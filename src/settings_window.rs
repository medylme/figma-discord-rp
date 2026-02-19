use crate::settings::{AppName, STATE_ENTRIES, Settings};
use eframe::egui;
use std::{
    process::Command,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub fn open(settings: Arc<RwLock<Settings>>, open_flag: Arc<AtomicBool>) {
    if open_flag.swap(true, Ordering::Relaxed) {
        return;
    }

    thread::spawn(move || {
        let exe = std::env::current_exe().expect("failed to resolve current exe");
        match Command::new(exe).arg("--settings").status() {
            Ok(status) if status.success() => {
                let reloaded = Settings::load();
                *settings.write().unwrap() = reloaded;
            }
            Ok(status) => eprintln!("[settings] window exited with {status}"),
            Err(e) => eprintln!("[settings] failed to spawn window: {e}"),
        }
        open_flag.store(false, Ordering::Relaxed);
    });
}

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/favicon.png"))
        .expect("failed to load window icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

pub fn run() {
    let draft = Settings::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 580.0])
            .with_resizable(false)
            .with_title("Figma Rich Presence")
            .with_icon(load_icon()),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "dyl-figma-discord-rp-settings",
        options,
        Box::new(move |cc| {
            configure_style(&cc.egui_ctx);
            let custom_name_buf = match &draft.app_name {
                AppName::Custom(s) => s.clone(),
                _ => String::new(),
            };
            Ok(Box::new(SettingsWindow {
                draft,
                custom_name_buf,
            }))
        }),
    ) {
        eprintln!("[settings] window error: {e}");
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_rounding = egui::Rounding::same(12.0);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}

struct SettingsWindow {
    draft: Settings,
    custom_name_buf: String,
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Settings");
                ui.add_space(8.0);

                ui.strong("App Name");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let is_custom = matches!(self.draft.app_name, AppName::Custom(_));
                    if ui
                        .radio(self.draft.app_name == AppName::Figma, "Figma")
                        .clicked()
                    {
                        self.draft.app_name = AppName::Figma;
                    }
                    if ui
                        .radio(
                            self.draft.app_name == AppName::FigmaDesktop,
                            "Figma Desktop",
                        )
                        .clicked()
                    {
                        self.draft.app_name = AppName::FigmaDesktop;
                    }
                    if ui.radio(is_custom, "Custom").clicked() {
                        self.draft.app_name = AppName::Custom(self.custom_name_buf.clone());
                    }
                    if ui
                        .add_enabled(
                            is_custom,
                            egui::TextEdit::singleline(&mut self.custom_name_buf)
                                .hint_text("App name")
                                .desired_width(140.0),
                        )
                        .changed()
                        && is_custom
                    {
                        self.draft.app_name = AppName::Custom(self.custom_name_buf.clone());
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.strong("Other Settings");
                ui.add_space(4.0);
                ui.checkbox(&mut self.draft.hide_filename, "Hide File Names");
                ui.checkbox(&mut self.draft.disable_idle, "Disable Idle Detection");
                ui.separator();
                ui.strong("Activity Images");
                ui.add_space(4.0);
                ui.label("Default Image URL");
                ui.add(
                    egui::TextEdit::singleline(&mut self.draft.default_image)
                        .hint_text("Default Figma Icon"),
                );
                ui.add_space(4.0);

                egui::Grid::new("overrides_grid")
                    .num_columns(2)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for &(key, display) in STATE_ENTRIES {
                            let entry = self
                                .draft
                                .image_overrides
                                .entry(key.to_string())
                                .or_default();
                            ui.checkbox(&mut entry.enabled, display);
                            ui.add_enabled(
                                entry.enabled,
                                egui::TextEdit::singleline(&mut entry.image_url)
                                    .hint_text("Image URL")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("Save").clicked() {
                        self.draft.save();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });
    }
}
