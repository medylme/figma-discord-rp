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

mod settings;
mod settings_window;
use settings::Settings;

mod tray;
use tray::TrayApp;

use crate::figma::is_figma_focused;

// windows - prevent opening console
#[cfg(target_os = "windows")]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

const FIGMA_POLLING_RATE_SECONDS: u64 = 5;
const RP_UPDATE_RATE_SECONDS: u64 = 5; // should be 15 but i don't give a frick!!!!

fn main() {
    #[cfg(target_os = "windows")]
    attach_parent_console();

    if std::env::args().any(|a| a == "--settings") {
        settings_window::run();
        return;
    }

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
                match scan_figma_active_tab() {
                    Ok(new_tab) => {
                        if !figma_connected.swap(true, Ordering::Relaxed) {
                            println!("[figma] connected");
                        }
                        let mut state = figma_state.write().unwrap();

                        if new_tab != state.active_tab {
                            state.active_tab = new_tab;
                        }
                        if is_figma_focused() {
                            state.last_focused_at = Some(Instant::now());
                        }
                    }
                    Err(e) => {
                        if figma_connected.swap(false, Ordering::Relaxed) {
                            eprintln!("[figma] disconnected: {e}");
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
                        println!("[discord] connected");
                        break;
                    }
                    Err(e) => {
                        eprintln!(
                            "[discord] connect failed: {e}, retrying in {RP_UPDATE_RATE_SECONDS}s"
                        );
                        thread::sleep(Duration::from_secs(RP_UPDATE_RATE_SECONDS));
                    }
                }
            }

            let mut session_start: Option<i64> = None;
            let mut was_figma_connected = false;

            while running.load(Ordering::Relaxed) {
                let figma_up = figma_connected.load(Ordering::Relaxed);

                if !figma_up {
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
                        .title
                        .clone()
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
                    eprintln!("[discord] failed to set activity: {e}, reconnecting");

                    loop {
                        if !running.load(Ordering::Relaxed) {
                            return;
                        }
                        match client.reconnect() {
                            Ok(_) => {
                                discord_connected.store(true, Ordering::Relaxed);
                                println!("[discord] reconnected");
                                break;
                            }
                            Err(e) => {
                                eprintln!(
                                    "[discord] reconnect failed: {e}, retrying in {RP_UPDATE_RATE_SECONDS}s"
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
