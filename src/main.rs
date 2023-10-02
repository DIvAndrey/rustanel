// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod highlighting;
pub mod compiler;
pub mod instruction_set;
mod executor;

use eframe::egui;
use crate::compiler::{Compiler, ErrorsHighlightInfo};
use crate::highlighting::{CodeTheme, highlight};

// fn main() {
//     dbg!(regex_captures!(r"^p([0-9]|10|11|12|13|14|15)$", "p115"));
// }

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
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
a:".into(),
            compiler: Compiler::build(),
        }
    }
}

impl MyApp {
    fn code_editor_ui(&mut self, ui: &mut egui::Ui, theme: &CodeTheme, errors: &ErrorsHighlightInfo) {
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
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(3.0);
        let errors = self.compiler.compile_code(&self.code);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut theme = CodeTheme::from_memory(ui.ctx());
            theme.ui(ui);
            theme.clone().store_in_memory(ui.ctx());
            ui.horizontal_top(|ui| {
                self.code_editor_ui(ui, &theme, &errors);
                let mut program_text = String::with_capacity(3000);
                for i in 0..self.compiler.program.len() {
                    program_text += format!("{:#04x}", self.compiler.program[i])[2..].to_ascii_uppercase().as_str();
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
pub fn code_view_ui(
    ui: &mut egui::Ui,
    mut code: &str,
) {
    ui.push_id(99999, |ui| {
        egui::ScrollArea::vertical().min_scrolled_height(ui.available_height()).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut code)
                    .code_editor()
                    .desired_rows(1)
            );
        });
    });
}
