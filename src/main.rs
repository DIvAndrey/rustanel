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
use eframe::egui::{include_image, vec2, RichText, Vec2, Visuals};

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
    new_pixels_per_point: f32,
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
            new_pixels_per_point: 2.0,
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
            if ui.visuals().dark_mode {
                include_image!("../data/on_dark.png")
            } else {
                include_image!("../data/on_light.png")
            }
        } else {
            if ui.visuals().dark_mode {
                include_image!("../data/off_dark.png")
            } else {
                include_image!("../data/off_light.png")
            }
        };
        ui.add(
            egui::ImageButton::new(
                egui::Image::new(image).fit_to_exact_size(Vec2::splat(lamp_size)),
            )
            .frame(false),
        )
    }

    fn get_monospace(text: &str, size: f32) -> RichText {
        RichText::new(text).size(size).monospace()
    }

    fn light_bulbs_panel_ui(&mut self, ui: &mut egui::Ui, lamp_size: f32) {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::Grid::new("Panel with light bulbs")
            .min_col_width(0.0)
            .min_row_height(0.0)
            .show(ui, |ui| {
                for i in 0..16 {
                    let space = if i > 9 { "" } else { " " };
                    ui.label(Self::get_monospace(
                        &format!("{space}P{i}"),
                        lamp_size * 0.7,
                    ));
                    ui.add_space(2.0);
                    for j in 0..16 {
                        let response = self.draw_lamp(
                            ui,
                            lamp_size,
                            (self.program_executor.display[i] >> (15 - j)) & 1 == 1,
                        );
                        if response.clicked() {
                            self.program_executor.display[i] ^= 1 << (15 - j);
                        }
                    }
                    ui.end_row();
                }
            });
        ui.spacing_mut().item_spacing = vec2(8.0, 3.0);
    }

    fn settings_and_info_panel_ui(&mut self, ui: &mut egui::Ui) {
        let mut is_dark_mode = ui.ctx().style().visuals.dark_mode;
        ui.horizontal(|ui| {
            ui.label("Theme:");
            ui.selectable_value(&mut is_dark_mode, false, "☀ Light");
            ui.selectable_value(&mut is_dark_mode, true, "🌙 Dark");
            ui.ctx().set_visuals(if is_dark_mode {
                Visuals::dark()
            } else {
                Visuals::light()
            });
            ui.add(egui::Separator::default().vertical());
            ui.label("Ui scale:");
            let response = ui.add(egui::Slider::new(&mut self.new_pixels_per_point, 0.5..=4.0));
            if !response.is_pointer_button_down_on() {
                ui.ctx().set_pixels_per_point(self.new_pixels_per_point);
            }
        });
        ui.separator();
        ui.label(RichText::new("Registers").strong().size(14.0));
        ui.end_row();
        egui::Grid::new("Settings and info")
            .min_row_height(0.0)
            .min_col_width(0.0)
            .spacing(vec2(14.0, 4.0))
            .show(ui, |ui| {
                ui.label(RichText::new("").strong().size(12.0));
                ui.label(RichText::new("binary").size(12.0));
                ui.label(RichText::new("hex").size(12.0));
                ui.label(RichText::new("unsigned").size(12.0));
                ui.label(RichText::new("signed").size(12.0));
                ui.end_row();
                for (i, &val) in self.program_executor.registers.iter().enumerate() {
                    let bits = format!("{val:#018b}")[2..].to_string();
                    let bits = format!(
                        "{} {} {} {}",
                        &bits[0..4],
                        &bits[4..8],
                        &bits[8..12],
                        &bits[12..16]
                    );
                    let hex = format!("{val:#06x}")[2..].to_string();
                    let unsigned = val.to_string();
                    let signed = (val as i16).to_string();
                    ui.label(Self::get_monospace(&format!("R{i}"), 10.0).strong());
                    ui.label(Self::get_monospace(&bits, 12.0));
                    ui.label(Self::get_monospace(&hex, 12.0));
                    ui.label(Self::get_monospace(&unsigned, 12.0));
                    ui.label(Self::get_monospace(&signed, 12.0));
                    ui.end_row()
                }
            });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = CodeTheme::from_memory(ctx);
        egui_extras::install_image_loaders(ctx);
        let errors = self.compiler.compile_code(&self.code);
        egui::TopBottomPanel::top("Light bulbs and registers")
            .resizable(true)
            .default_height(256.0)
            .show(ctx, |ui| {
                let available = ui.available_size();
                let panel_size = available.y.min(available.x * 0.5);
                let ppp = ui.ctx().pixels_per_point();
                let lamp_size = (panel_size * ppp / 16.0).round() / ppp;
                ui.horizontal_top(|ui| {
                    self.light_bulbs_panel_ui(ui, lamp_size);
                    ui.add(egui::Separator::default().vertical().spacing(10.0));
                    ui.vertical(|ui| {
                        self.settings_and_info_panel_ui(ui);
                        theme.apply_bg_color(ui);
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
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
