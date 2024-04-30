// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod compiler;
mod executor;
mod highlighting;
pub mod instruction_set;

use crate::compiler::{CompilationError, Compiler, ErrorsHighlightInfo, MAX_PROGRAM_SIZE};
use crate::executor::{ProgramExecutor, RuntimeError};
use crate::highlighting::{highlight, CodeTheme, TokenType};
use eframe::egui;
use eframe::egui::{include_image, vec2, Align2, Color32, RichText, Vec2, Visuals, Widget};
use eframe::epaint::text::LayoutJob;
use std::ops::Range;
use lazy_regex::{regex_captures, regex_is_match};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(vec2(1280.0, 960.0)),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Rustanel – a rusty panel with light bulbs",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

pub enum ErrorPopupInfo {
    CompilationError(CompilationError),
    RuntimeError(RuntimeError),
    None,
}

struct MyApp {
    code: String,
    compiler: Compiler,
    program_executor: ProgramExecutor,
    new_pixels_per_point: f32,
    last_info_panel_height: f32,
    error_popup_info: ErrorPopupInfo,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            code: "\
mov r0, -1
a:
add r0, 1
jmp @a
stop
"
            .into(),
            compiler: Compiler::build(),
            program_executor: ProgramExecutor::new(),
            new_pixels_per_point: 2.5,
            last_info_panel_height: 0.0,
            error_popup_info: ErrorPopupInfo::None,
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

    fn draw_register_info_row(&mut self, ui: &mut egui::Ui, name: &str, val: u16) {
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
        ui.label(Self::get_monospace(name, 10.0).strong());
        ui.label(Self::get_monospace(&bits, 10.0));
        ui.label(Self::get_monospace(&hex, 10.0));
        ui.label(Self::get_monospace(&unsigned, 10.0));
        ui.label(Self::get_monospace(&signed, 10.0));
        ui.end_row()
    }

    fn draw_registers_grid(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("reg").size(8.0));
        ui.label(RichText::new("binary").size(8.0));
        ui.label(RichText::new("hex").size(8.0));
        ui.label(RichText::new("unsigned").size(8.0));
        ui.label(RichText::new("signed").size(8.0));
        ui.end_row();
        for i in 0..4 {
            self.draw_register_info_row(ui, &format!("R{i}"), self.program_executor.registers[i]);
        }
        ui.end_row();
        self.draw_register_info_row(ui, "PC", self.program_executor.curr_addr as u16);
        self.draw_register_info_row(ui, "SP", self.program_executor.registers[4]);
        self.draw_register_info_row(ui, "PS", self.program_executor.program_state_reg);
    }

    fn execute_next_instruction(&mut self) {
        self.error_popup_info = ErrorPopupInfo::None;
        match self.program_executor.execute_next_instruction() {
            Err(err) => {
                self.program_executor.has_finished = true;
                self.error_popup_info = ErrorPopupInfo::RuntimeError(err);
            }
            Ok(_) => {}
        };
    }

    fn are_there_compilation_errors(&mut self) -> bool {
        if let Some(err) = self.compiler.errors.first() {
            self.program_executor.has_finished = true;
            self.error_popup_info = ErrorPopupInfo::CompilationError(err.1.clone());
            return true;
        }
        false
    }

    fn build_run_debug_buttons(&mut self, ui: &mut egui::Ui) {
        if ui.button("Build").clicked() {
            self.program_executor.is_in_debug_mode = false;
            self.program_executor.has_finished = true;
            if !self.are_there_compilation_errors() {
                self.program_executor.memory = self.compiler.program.clone();
            }
        }
        if ui.button("Run").clicked() {
            self.program_executor.is_in_debug_mode = false;
            self.program_executor.prepare_for_a_new_run();
            if !self.are_there_compilation_errors() {
                self.program_executor.memory = self.compiler.program.clone();
            }
            self.execute_next_instruction();
        }
        if ui.button("Step over").clicked() {
            if (self.program_executor.has_finished || !self.program_executor.is_in_debug_mode)
                && !self.are_there_compilation_errors()
            {
                self.program_executor.is_in_debug_mode = true;
                self.program_executor.prepare_for_a_new_run();
                if !self.are_there_compilation_errors() {
                    self.program_executor.memory = self.compiler.program.clone();
                }
            } else {
                self.execute_next_instruction();
            }
        }
        if ui.button("Clear registers").clicked() {
            for i in 0..5 {
                self.program_executor.registers[i] = 0;
            }
        }
        if !self.program_executor.has_finished && !self.program_executor.is_in_debug_mode {
            self.execute_next_instruction();
        }
    }

    fn settings_and_info_panel_ui(&mut self, ui: &mut egui::Ui, errors: &ErrorsHighlightInfo) {
        let mut is_dark_mode = ui.ctx().style().visuals.dark_mode;
        ui.horizontal(|ui| {
            ui.label("Ui scale:");
            let response = ui.add(egui::Slider::new(&mut self.new_pixels_per_point, 0.5..=4.0));
            if !response.is_pointer_button_down_on() {
                ui.ctx().set_pixels_per_point(self.new_pixels_per_point);
            }
            ui.separator();
            ui.label("Theme:");
            ui.selectable_value(&mut is_dark_mode, false, "☀ Light");
            ui.selectable_value(&mut is_dark_mode, true, "🌙 Dark");
            ui.ctx().set_visuals(if is_dark_mode {
                Visuals::dark()
            } else {
                Visuals::light()
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Registers").strong().size(14.0));
            ui.separator();
            self.build_run_debug_buttons(ui);
        });
        ui.end_row();
        egui::Grid::new("Settings and info")
            .min_row_height(0.0)
            .min_col_width(0.0)
            .spacing(vec2(12.0, 4.0))
            .show(ui, |ui| {
                self.draw_registers_grid(ui);
            });
        self.error_messages_list_ui(ui, &errors);
    }

    fn error_messages_list_ui(&mut self, ui: &mut egui::Ui, errors: &ErrorsHighlightInfo) {
        let mut error_messages: Vec<String> =
            errors.iter().map(|(_, err)| format!("{err}")).collect();
        if error_messages.is_empty() {
            return;
        }
        error_messages.sort_unstable();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::TextEdit::multiline(&mut error_messages.join("\n").as_str())
                .desired_rows(0)
                .ui(ui);
        });
    }

    fn show_error_popup(&mut self, ctx: &egui::Context) {
        let (title, text) = match &self.error_popup_info {
            ErrorPopupInfo::None => return,
            ErrorPopupInfo::CompilationError(err) => ("Compilation error", err.to_string()),
            ErrorPopupInfo::RuntimeError(err) => ("Runtime error", err.to_string()),
        };
        let mut is_opened = !matches!(&self.error_popup_info, ErrorPopupInfo::None);
        egui::Window::new(RichText::new(title).color(Color32::RED))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut is_opened)
            .show(ctx, |ui| {
                ui.label(text);
            });
        if !is_opened {
            self.error_popup_info = ErrorPopupInfo::None;
        }
    }

    fn get_binary_viewer_rows(&self, rows_range: Range<usize>, theme: &CodeTheme) -> LayoutJob {
        let mut layout_job = LayoutJob::default();
        layout_job.text.reserve(rows_range.len() * 8);
        let range = (rows_range.start * 8)..(rows_range.end * 8).min(MAX_PROGRAM_SIZE);
        let text_format = theme.formats[TokenType::Punctuation].clone();
        let highlighted_format = theme.formats[TokenType::Label].clone();
        for i in range.clone() {
            if (i & 0b111) == 0 {
                if i != range.start {
                    layout_job.append("\n", 0.0, text_format.clone());
                }
                layout_job.append(
                    &format!("{:#06x}: ", i).to_ascii_uppercase().as_str()[2..],
                    0.0,
                    text_format.clone(),
                );
            } else {
                layout_job.append(" ", 0.0, text_format.clone());
            }
            layout_job.append(
                &format!("{:#04x}", self.program_executor.memory[i]).to_ascii_uppercase()[2..],
                0.0,
                if i == self.program_executor.curr_addr && !self.program_executor.has_finished {
                    highlighted_format.clone()
                } else {
                    text_format.clone()
                },
            );
        }
        layout_job
    }

    fn binary_viewer_ui(&self, ui: &mut egui::Ui, theme: &CodeTheme) {
        ui.push_id("Binary code viewer", |ui| {
            egui::ScrollArea::vertical()
                .min_scrolled_height(ui.available_height())
                .show_rows(ui, 8.0, MAX_PROGRAM_SIZE / 8, |ui, rows_range| {
                    let mut layout_job =
                        self.get_binary_viewer_rows(rows_range.start..(rows_range.end + 5), theme);
                    ui.add(
                        egui::TextEdit::multiline(&mut layout_job.clone().text.as_str())
                            .layouter(&mut |ui: &egui::Ui, _: &str, wrap_width: f32| {
                                layout_job.wrap.max_width = wrap_width;
                                ui.fonts(|f| f.layout_job(layout_job.clone()))
                            })
                            .code_editor()
                            .desired_rows(1),
                    );
                });
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_error_popup(ctx);
        let theme = CodeTheme::from_memory(ctx);
        egui_extras::install_image_loaders(ctx);
        self.compiler.compile_code(&self.code);
        egui::TopBottomPanel::top("Light bulbs and registers")
            .resizable(true)
            .min_height(self.last_info_panel_height)
            .default_height(256.0)
            .show(ctx, |ui| {
                let available = ui.available_size();
                let panel_size = available.y.min(available.x * 0.5);
                let ppp = ui.ctx().pixels_per_point();
                let lamp_size = (panel_size * ppp / 16.0).round() / ppp;
                ui.horizontal_top(|ui| {
                    self.light_bulbs_panel_ui(ui, lamp_size);
                    ui.add(egui::Separator::default().vertical().spacing(10.0));
                    ui.horizontal_top(|ui| {
                        self.last_info_panel_height = ui
                            .vertical(|ui| {
                                self.settings_and_info_panel_ui(ui, &self.compiler.errors.clone());
                            })
                            .response
                            .rect
                            .height();
                        theme.apply_bg_color(ui);
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            theme.clone().store_in_memory(ui.ctx());
            ui.horizontal_top(|ui| {
                self.code_editor_ui(ui, &theme, &self.compiler.errors.clone());
                self.binary_viewer_ui(ui, &theme);
            });
        });
        if !self.program_executor.has_finished && !self.program_executor.is_in_debug_mode {
            ctx.request_repaint();
        }
    }
}
