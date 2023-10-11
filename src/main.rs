// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod compiler;
mod executor;
mod highlighting;
pub mod instruction_set;

use crate::compiler::{Compiler, ErrorsHighlightInfo};
use crate::executor::ProgramState;
use crate::highlighting::{highlight, CodeTheme};
use eframe::egui;
use eframe::egui::{include_image, vec2, Vec2, Widget};

// fn main() {
//     dbg!(regex_captures!(r"^p([0-9]|10|11|12|13|14|15)$", "p115"));
// }

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(vec2(1280.0, 960.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Rustanel – a rusty panel with light bulbs",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    code: String,
    compiler: Compiler,
    program_executor: ProgramState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            code: "\
mov (r0), (r1)+
mov r0, @b
mov r0, 1
mov r0, @b
stop
a:"
            .into(),
            compiler: Compiler::build(),
            program_executor: ProgramState::new(),
        }
    }
}

impl MyApp {
    fn code_editor_ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: &CodeTheme,
        errors: &ErrorsHighlightInfo,
    ) {
        theme.apply_bg_color(ui);
        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight(ui.ctx(), theme, string, errors);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.code)
                    .code_editor()
                    .desired_rows(1)
                    .desired_width(ui.available_width() * 0.5)
                    .layouter(&mut layouter),
            );
        });
    }

    fn draw_lamp(&mut self, ui: &mut egui::Ui, lamp_size: f32, enabled: bool) -> egui::Response {
        let image = if enabled {
            include_image!("../data/on.png")
        } else {
            include_image!("../data/off.png")
        };
        ui.add(
            egui::ImageButton::new(egui::Image::new(image).fit_to_exact_size(Vec2::splat(
                lamp_size,
            )))
            .frame(false),
        )
    }

    fn panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let available = ui.available_size();
        let panel_size = available.y.min(available.x * 0.5);
        let ppp = ui.ctx().pixels_per_point();
        let lamp_size = (panel_size * ppp / 16.0).round() / ppp;
        egui::Grid::new("Panel with light bulbs")
            .min_col_width(0.0)
            .min_row_height(0.0)
            .show(ui, |ui| {
                for i in 0..16 {
                    for j in 0..16 {
                        let response = self
                            .draw_lamp(ui, lamp_size, (self.program_executor.display[i] >> (15 - j)) & 1 == 1);
                        if response.clicked() {
                            self.program_executor.display[i] ^= 1 << (15 - j);
                        }
                    }
                    ui.end_row();
                }
            });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        ctx.set_pixels_per_point(2.2323423443);
        let errors = self.compiler.compile_code(&self.code);
        egui::TopBottomPanel::top("Light bulbs and registers")
            .resizable(true)
            .default_height(256.0)
            .show(ctx, |ui| {
                self.panel_ui(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut theme = CodeTheme::from_memory(ui.ctx());
            ui.horizontal_top(|ui| {
                theme.ui(ui);
            });
            theme.clone().store_in_memory(ui.ctx());
            ui.horizontal_top(|ui| {
                self.code_editor_ui(ui, &theme, &errors);
                let mut program_text = String::with_capacity(12288);
                for i in 0..self.compiler.program.len() {
                    program_text += format!("{:#04x}", self.compiler.program[i])[2..]
                        .to_ascii_uppercase()
                        .as_str();
                    if (i & 0b111) == 0b111 {
                        program_text += "\n";
                    } else {
                        program_text += " ";
                    }
                }
                code_view_ui(ui, &mut program_text);
            });
        });
    }
}

/// View some code with syntax highlighting and selection.
pub fn code_view_ui(ui: &mut egui::Ui, mut code: &str) {
    ui.push_id(99999, |ui| {
        egui::ScrollArea::vertical()
            .min_scrolled_height(ui.available_height())
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut code)
                        .code_editor()
                        .desired_rows(1),
                );
            });
    });
}
