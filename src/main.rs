#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use dotenvy_macro::dotenv;
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use winit::event_loop::EventLoop;

mod figma;
use figma::{FigmaState, scan_figma_active_tab};

mod logging;

mod settings;
mod settings_window;
use settings::Settings;

mod tray;
use tray::TrayApp;

mod updater;
use updater::core::{is_auto_update_enabled, set_auto_update_enabled};

use crate::figma::{find_figma_pids, is_figma_focused};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_os = "windows")]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, SetConsoleCtrlHandler,
    };
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);

        unsafe extern "system" fn ctrl_handler(_ctrl_type: u32) -> i32 {
            std::process::exit(1)
        }
        SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    }
}

const FIGMA_POLLING_RATE_SECONDS: u64 = 5;
const RP_UPDATE_RATE_SECONDS: u64 = 15;

fn main() {
    #[cfg(target_os = "windows")]
    attach_parent_console();

    set_auto_update_enabled(!std::env::args().any(|a| a == "--no-update"));

    if std::env::args().any(|a| a == "--settings") {
        settings_window::run();
        return;
    }

    if is_auto_update_enabled() {
        updater::install::cleanup_old_binary();
        let _ = updater::splash::run_startup_update_check();
    }

    log_info!("main", "Starting figma-discord-rp v{}", VERSION);
    log_debug!("main", "Auto-update enabled: {}", is_auto_update_enabled());

    let running = Arc::new(AtomicBool::new(true));
    let figma_state = Arc::new(RwLock::new(FigmaState::default()));
    let figma_connected = Arc::new(AtomicBool::new(false));
    let discord_connected = Arc::new(AtomicBool::new(false));
    let settings = Arc::new(RwLock::new(Settings::load()));

    thread::spawn({
        let figma_state = Arc::clone(&figma_state);
        let figma_connected = Arc::clone(&figma_connected);
        let running = Arc::clone(&running);
        move || {
            while running.load(Ordering::Relaxed) {
                let pids = find_figma_pids();
                if pids.is_empty() {
                    if figma_connected.swap(false, Ordering::Relaxed) {
                        log_warn!("figma", "Process not found, disconnecting");
                        let mut state = figma_state.write().unwrap();
                        *state = FigmaState::default();
                    }
                    thread::sleep(Duration::from_secs(FIGMA_POLLING_RATE_SECONDS));
                    continue;
                }

                match scan_figma_active_tab() {
                    Ok(new_tab) => {
                        if !figma_connected.swap(true, Ordering::Relaxed) {
                            log_info!("figma", "Connected");
                            let pid_list: Vec<String> =
                                pids.iter().map(|p| p.to_string()).collect();
                            log_debug!(
                                "figma",
                                "{} process(es): {}",
                                pids.len(),
                                pid_list.join(", ")
                            );
                        }
                        let mut state = figma_state.write().unwrap();

                        if new_tab != state.active_tab {
                            let title = new_tab
                                .as_ref()
                                .and_then(|t| t.title.as_deref())
                                .unwrap_or("None");
                            let editor = new_tab
                                .as_ref()
                                .and_then(|t| t.editor_type.as_ref())
                                .map(|e| e.key())
                                .unwrap_or("none");
                            log_debug!("figma", "Tab changed: \"{}\" ({})", title, editor);
                            state.active_tab = new_tab;
                        }
                        if is_figma_focused(&pids) {
                            state.last_focused_at = Some(Instant::now());
                        }
                    }
                    Err(e) => {
                        if figma_connected.swap(false, Ordering::Relaxed) {
                            log_warn!("figma", "Disconnected: {e}");
                            let mut state = figma_state.write().unwrap();
                            *state = FigmaState::default();
                        }
                    }
                }
                thread::sleep(Duration::from_secs(FIGMA_POLLING_RATE_SECONDS));
            }
        }
    });

    thread::spawn({
        let figma_state = Arc::clone(&figma_state);
        let figma_connected = Arc::clone(&figma_connected);
        let discord_connected = Arc::clone(&discord_connected);
        let settings = Arc::clone(&settings);
        let running = Arc::clone(&running);
        move || {
            let mut client = DiscordIpcClient::new(dotenv!("DISCORD_APP_ID"));

            loop {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                match client.connect() {
                    Ok(_) => {
                        discord_connected.store(true, Ordering::Relaxed);
                        log_info!("discord", "Connected");
                        log_debug!("discord", "Self pid: {}", std::process::id());
                        break;
                    }
                    Err(e) => {
                        log_error!(
                            "discord",
                            "Connect failed: {e}, retrying in {RP_UPDATE_RATE_SECONDS}s"
                        );
                        thread::sleep(Duration::from_secs(RP_UPDATE_RATE_SECONDS));
                    }
                }
            }

            let mut session_start: Option<i64> = None;
            let mut was_figma_connected = false;

            while running.load(Ordering::Relaxed) {
                let figma_up = figma_connected.load(Ordering::Relaxed);

                let has_active_tab = figma_up && figma_state.read().unwrap().active_tab.is_some();

                if !figma_up || !has_active_tab {
                    let _ = client.clear_activity();
                    was_figma_connected = false;
                    session_start = None;
                    thread::sleep(Duration::from_secs(RP_UPDATE_RATE_SECONDS));
                    continue;
                }

                if !was_figma_connected {
                    session_start = Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                    );
                    was_figma_connected = true;
                }

                let (title, status, state_key) = {
                    let figma = figma_state.read().unwrap();
                    let disable_idle = settings.read().unwrap().disable_idle;
                    let title = figma
                        .active_tab
                        .as_ref()
                        .and_then(|t| t.title.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let (status, state_key) = if figma.is_idle() && !disable_idle {
                        ("Idle".to_string(), "idle".to_string())
                    } else {
                        (figma.status(), figma.state_key().to_string())
                    };
                    (title, status, state_key)
                };

                let (image_url, app_name, hide_filename) = {
                    let s = settings.read().unwrap();
                    (
                        s.image_url_for_state(&state_key).to_string(),
                        s.resolved_app_name().to_string(),
                        s.hide_filename,
                    )
                };

                let details = match hide_filename {
                    true => None,
                    false => Some(format!("File: {title}")),
                };
                log_debug!(
                    "discord",
                    "Setting activity: status={status}, app={app_name}, image={image_url}"
                );

                let assets = activity::Assets::new().large_image(&image_url);
                let timestamps = activity::Timestamps::new().start(session_start.unwrap());

                let mut activity = activity::Activity::new()
                    .name(&app_name)
                    .state(&status)
                    .assets(assets)
                    .timestamps(timestamps);

                if let Some(ref d) = details {
                    activity = activity.details(d);
                }

                if let Err(e) = client.set_activity(activity) {
                    discord_connected.store(false, Ordering::Relaxed);
                    log_error!("discord", "Failed to set activity: {e}, reconnecting");

                    loop {
                        if !running.load(Ordering::Relaxed) {
                            return;
                        }
                        match client.reconnect() {
                            Ok(_) => {
                                discord_connected.store(true, Ordering::Relaxed);
                                log_info!("discord", "Reconnected");
                                break;
                            }
                            Err(e) => {
                                log_error!(
                                    "discord",
                                    "Reconnect failed: {e}, retrying in {RP_UPDATE_RATE_SECONDS}s"
                                );
                                thread::sleep(Duration::from_secs(RP_UPDATE_RATE_SECONDS));
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_secs(RP_UPDATE_RATE_SECONDS));
            }
        }
    });

    let event_loop = EventLoop::new().unwrap();
    let mut app = TrayApp::new(
        running,
        figma_state,
        figma_connected,
        discord_connected,
        settings,
    );
    event_loop.run_app(&mut app).unwrap();
}
