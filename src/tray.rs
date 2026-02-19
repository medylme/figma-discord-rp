use crate::figma::FigmaState;
use crate::settings::Settings;
use crate::settings_window;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tray_icon::{TrayIcon, TrayIconBuilder};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

fn make_icon() -> tray_icon::Icon {
    let image = image::load_from_memory(include_bytes!("../assets/favicon.png"))
        .expect("failed to load tray icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height).expect("failed to create tray icon")
}

pub struct TrayApp {
    pub running: Arc<AtomicBool>,
    figma_connected: Arc<AtomicBool>,
    discord_connected: Arc<AtomicBool>,
    settings: Arc<RwLock<Settings>>,
    settings_open: Arc<AtomicBool>,
    quit_item: MenuItem,
    settings_item: MenuItem,
    figma_status: MenuItem,
    discord_status: MenuItem,
    tray: Option<TrayIcon>,
    figma_state: Arc<RwLock<FigmaState>>,
}

impl TrayApp {
    pub fn new(
        running: Arc<AtomicBool>,
        figma_state: Arc<RwLock<FigmaState>>,
        figma_connected: Arc<AtomicBool>,
        discord_connected: Arc<AtomicBool>,
        settings: Arc<RwLock<Settings>>,
    ) -> Self {
        Self {
            running,
            figma_connected,
            discord_connected,
            settings,
            settings_open: Arc::new(AtomicBool::new(false)),
            quit_item: MenuItem::new("Quit", true, None),
            settings_item: MenuItem::new("Settings...", true, None),
            figma_status: MenuItem::new("Figma: Connecting...", false, None),
            discord_status: MenuItem::new("Discord: Connecting...", false, None),
            tray: None,
            figma_state,
        }
    }

    fn init_tray(&mut self) {
        let title = MenuItem::new("Figma Rich Presence", false, None);
        let menu = Menu::new();
        menu.append(&title).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.discord_status).unwrap();
        menu.append(&self.figma_status).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.settings_item).unwrap();
        menu.append(&self.quit_item).unwrap();
        self.tray = Some(
            TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("Figma Rich Presence")
                .with_icon(make_icon())
                .build()
                .unwrap(),
        );
    }

    fn update_status_items(&self) {
        let figma_text = if self.figma_connected.load(Ordering::Relaxed) {
            "Figma: Connected"
        } else {
            "Figma: Disconnected"
        };
        let discord_text = if self.discord_connected.load(Ordering::Relaxed) {
            "Discord: Connected"
        } else {
            "Discord: Disconnected"
        };
        self.figma_status.set_text(figma_text);
        self.discord_status.set_text(discord_text);
    }

    fn update_tooltip(&self) {
        let Some(tray) = &self.tray else { return };
        let state = self.figma_state.read().unwrap();
        let title = state.active_tab.title.as_deref().unwrap_or("No file open");
        let status = state.status();
        let tooltip = format!("Figma Rich Presence — {status}: {title}");
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.tray.is_none() {
            self.init_tray();
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(200),
        ));

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id() == self.quit_item.id() {
                self.running.store(false, Ordering::Relaxed);
                event_loop.exit();
            } else if event.id() == self.settings_item.id() {
                settings_window::open(Arc::clone(&self.settings), Arc::clone(&self.settings_open));
            }
        }

        self.update_status_items();
        self.update_tooltip();
    }
}
