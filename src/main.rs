// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod highlighting;
pub mod compiler;

use eframe::egui;
use eframe::egui::Key::P;
use crate::highlighting::{CodeTheme, highlight};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    code: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            code: "mov eax, 5\nadd r0, r1".into(),
        }
    }
}

impl MyApp {
    fn code_editor(&mut self, ui: &mut egui::Ui, theme: &CodeTheme) {
        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight(ui.ctx(), theme, string);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.code)
                    .font(egui::TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .desired_rows(1)
                    .lock_focus(true)
                    .desired_width(ui.available_width() * 0.5)
                    .layouter(&mut layouter),
            );
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(3.0);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            let mut theme = CodeTheme::from_memory(ui.ctx());
            ui.collapsing("Theme", |ui| {
                ui.group(|ui| {
                    theme.ui(ui);
                    theme.clone().store_in_memory(ui.ctx());
                });
            });
            ui.horizontal(|ui| {
                self.code_editor(ui, &theme);
                code_view_ui(ui, &theme, &self.code);
            });
        });
    }
}

/// View some code with syntax highlighting and selection.
pub fn code_view_ui(
    ui: &mut egui::Ui,
    theme: &CodeTheme,
    mut code: &str,
) -> egui::Response {
    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
        let layout_job = highlight(ui.ctx(), theme, string);
        // layout_job.wrap.max_width = wrap_width; // no wrapping
        ui.fonts(|f| f.layout_job(layout_job))
    };

    ui.add(
        egui::TextEdit::multiline(&mut code)
            .font(egui::TextStyle::Monospace) // for cursor height
            .code_editor()
            .desired_rows(1)
            .lock_focus(true)
            .layouter(&mut layouter),
    )
}

