// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod highlighting;
pub mod compiler;
pub mod instruction_set;
mod executor;

use eframe::egui;
use eframe::egui::{include_image, lerp, pos2, Rounding, TextureOptions, vec2, Vec2, Widget, WidgetInfo, widgets, WidgetType};
use crate::compiler::{Compiler, ErrorsHighlightInfo};
use crate::executor::ProgramState;
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
a:".into(),
            compiler: Compiler::build(),
            program_executor: ProgramState::new(),
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

    // fn draw_image_button_at(&mut self, ui: &mut egui::Ui, image: egui::Image, rect: egui::Rect) {
    //     let padding = Vec2::ZERO;
    //
    //     let available_size_for_image = ui.available_size() - 2.0 * padding;
    //     let tlr = image.load_for_size(ui.ctx(), available_size_for_image);
    //     let original_image_size = tlr.as_ref().ok().and_then(|t| t.size());
    //     let image_size = image
    //         .calc_size(available_size_for_image, original_image_size);
    //
    //     let padded_size = image_size + 2.0 * padding;
    //     let (rect, response) = ui.allocate_exact_size(padded_size, egui::Sense::click());
    //     response.widget_info(|| WidgetInfo::new(WidgetType::ImageButton));
    //
    //     if ui.is_rect_visible(rect) {
    //         let (expansion, rounding, fill, stroke) = if false {
    //             let selection = ui.visuals().selection;
    //             (
    //                 Vec2::ZERO,
    //                 Rounding::ZERO,
    //                 selection.bg_fill,
    //                 selection.stroke,
    //             )
    //         } else {
    //             Default::default()
    //         };
    //
    //         // Draw frame background (for transparent images):
    //         ui.painter()
    //             .rect_filled(rect.expand2(expansion), rounding, fill);
    //
    //         let image_rect = ui
    //             .layout()
    //             .align_size_within_rect(image_size, rect.shrink2(padding));
    //         // let image_rect = image_rect.expand2(expansion); // can make it blurry, so let's not
    //         let image_options = egui::ImageOptions {
    //             rounding, // apply rounding to the image
    //             ..image.image_options().clone()
    //         };
    //         widgets::image::paint_texture_load_result(ui, &tlr, image_rect, None, &image_options);
    //
    //         // Draw frame outline:
    //         ui.painter()
    //             .rect_stroke(rect.expand2(expansion), rounding, stroke);
    //     }
    // }

    fn panel_ui(&mut self, ui: &mut egui::Ui) {
        const PANEL_SIZE: f32 = 100.0;
        // const PANEL_SIZE: f32 = 150.0;
        // let test_size = egui::Image::new(include_image!("../data/off.png"))
        //     .fit_to_exact_size(vec2(PANEL_SIZE / 16.0, PANEL_SIZE / 16.0)).ui(ui).rect.size();
        // dbg!(test_size);
        // dbg!(PANEL_SIZE / 16.0);
        egui::Grid::new("Panel with light bulbs")
            .spacing(vec2(-40.0 + PANEL_SIZE / 15.0, -20.0 + PANEL_SIZE / 15.0))
            // .spacing(vec2(-33.0, -11.0))
            .show(ui, |ui| {
                for i in 0..16 {
                    for j in 0..16 {
                        egui::ImageButton::new(egui::Image::new(include_image!("../data/off.png"))
                            .fit_to_exact_size(vec2(PANEL_SIZE / 16.0, PANEL_SIZE / 16.0))
                        ).frame(false)
                            .ui(ui);
                    }
                    ui.end_row();
                }
        });
        // let rect = ui.allocate_space(vec2(PANEL_SIZE, PANEL_SIZE)).1;
        // let mut builder = egui_extras::TableBuilder::new(ui);
        // for i in 0..16 {
        //     builder = builder.body(|)
        //     for j in 0..16 {
        //
        //
        //         widgets::image::texture_load_result_response(self.image.source(), &tlr, response)
        //         egui::ImageButton::new(egui::Image::new(include_image!("../data/off.png"))
        //             .rounding(3.0)
        //             // .paint_at(ui, egui::Rect {
        //             //     min: pos2(
        //             //         lerp(rect.min.x..=rect.max.x, i as f32 / 16.0),
        //             //         lerp(rect.min.y..=rect.max.y, j as f32 / 16.0),
        //             //     ),
        //             //     max: pos2(
        //             //         lerp(rect.min.x..=rect.max.x, (i as f32 + 1.1) / 16.0),
        //             //         lerp(rect.min.y..=rect.max.y, (j as f32 + 1.1) / 16.0),
        //             //     ),
        //             // })
        //         ).ui(ui);
        //     }
        // }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        ctx.set_pixels_per_point(3.0);
        let errors = self.compiler.compile_code(&self.code);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut theme = CodeTheme::from_memory(ui.ctx());
            theme.ui(ui);
            self.panel_ui(ui);
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
