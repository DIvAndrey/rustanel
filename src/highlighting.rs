use crate::compiler::ErrorsHighlightInfo;
use eframe::egui;
use eframe::egui::{Button, Color32, Stroke, TextFormat, Visuals};
use eframe::epaint::ahash::HashSet;
use egui::text::LayoutJob;
use enum_map::Enum;
use lazy_static::lazy_static;
use crate::instruction_set::INSTRUCTION_SET;
use eframe::egui::ahash::HashSetExt;

/// Add syntax highlighting to a code string.
///
/// The results are memoized, so you can call this every frame without performance penalty.
pub fn highlight(
    ctx: &egui::Context,
    theme: &CodeTheme,
    code: &str,
    errors: &ErrorsHighlightInfo,
) -> LayoutJob {
    impl egui::util::cache::ComputerMut<(&CodeTheme, &str, &ErrorsHighlightInfo), LayoutJob>
        for Highlighter
    {
        fn compute(
            &mut self,
            (theme, code, errors): (&CodeTheme, &str, &ErrorsHighlightInfo),
        ) -> LayoutJob {
            self.highlight(theme, code, errors)
        }
    }

    type HighlightCache = egui::util::cache::FrameCache<LayoutJob, Highlighter>;

    ctx.memory_mut(|mem| {
        mem.caches
            .cache::<HighlightCache>()
            .get((theme, code, errors))
    })
}

#[derive(Clone, Copy, PartialEq, Enum)]
enum TokenType {
    Comment,
    Keyword,
    Literal,
    Number,
    StringLiteral,
    Punctuation,
    Whitespace,
}

/// A selected color theme.
#[derive(Clone, Hash, PartialEq)]
pub struct CodeTheme {
    dark_mode: bool,
    formats: enum_map::EnumMap<TokenType, egui::TextFormat>,
    bg_color: Color32,
    compiled_program: [u8; 0x100],
}

impl Default for CodeTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl CodeTheme {
    /// Load code theme from egui memory.
    ///
    /// There is one dark and one light theme stored at any one time.
    pub fn from_memory(ctx: &egui::Context) -> Self {
        if ctx.style().visuals.dark_mode {
            ctx.data_mut(|d| {
                d.get_persisted(egui::Id::new("dark"))
                    .unwrap_or_else(CodeTheme::dark)
            })
        } else {
            ctx.data_mut(|d| {
                d.get_persisted(egui::Id::new("light"))
                    .unwrap_or_else(CodeTheme::light)
            })
        }
    }

    /// Store theme to egui memory.
    ///
    /// There is one dark and one light theme stored at any one time.
    pub fn store_in_memory(self, ctx: &egui::Context) {
        if self.dark_mode {
            ctx.data_mut(|d| d.insert_persisted(egui::Id::new("dark"), self));
        } else {
            ctx.data_mut(|d| d.insert_persisted(egui::Id::new("light"), self));
        }
    }

    pub fn dark() -> Self {
        let font_id = egui::FontId::monospace(10.0);
        use egui::TextFormat;
        Self {
            dark_mode: true,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::from_gray(120)),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(207, 142, 109)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(192, 118, 172)),
                TokenType::Number => TextFormat::simple(font_id.clone(), Color32::from_rgb(42, 172, 184)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(105, 170, 111)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::LIGHT_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
            bg_color: Color32::from_rgb(30, 31, 34),
            compiled_program: [0; 0x0100],
        }
    }

    pub fn light() -> Self {
        let font_id = egui::FontId::monospace(10.0);
        use egui::TextFormat;

        Self {
            dark_mode: false,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::GRAY),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(235, 0, 0)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(153, 134, 255)),
                TokenType::Number => TextFormat::simple(font_id.clone(), Color32::from_rgb(42, 172, 184)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(105, 170, 111)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::DARK_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
            bg_color: Color32::from_gray(255),
            compiled_program: [0; 0x0100],
        }
    }

    pub fn apply_bg_color(&self, ui: &mut egui::Ui) {
        let mut old_visuals = ui.ctx().style().visuals.clone();
        old_visuals.extreme_bg_color = self.bg_color;
        old_visuals.code_bg_color = self.bg_color;
        ui.ctx().set_visuals(old_visuals);
    }

    fn light_dark_small_toggle_button(&mut self, ui: &mut egui::Ui) {
        if ui.visuals().dark_mode {
            if ui
                .add(Button::new("☀").frame(false))
                .on_hover_text("Switch to light mode")
                .clicked()
            {
                ui.ctx().set_visuals(Visuals::light());
            }
        } else {
            if ui
                .add(Button::new("🌙").frame(false))
                .on_hover_text("Switch to dark mode")
                .clicked()
            {
                ui.ctx().set_visuals(Visuals::dark());
            }
        }
    }

    /// Show UI for changing the color theme.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            let selected_id = egui::Id::null();
            let mut selected_tt: TokenType =
                ui.data_mut(|d| *d.get_persisted_mut_or(selected_id, TokenType::Comment));

            ui.vertical(|ui| {
                self.light_dark_small_toggle_button(ui);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.scope(|ui| {
                    for (tt, tt_name) in [
                        (TokenType::Comment, "// comment"),
                        (TokenType::Keyword, "keyword"),
                        (TokenType::Literal, "literal"),
                        (TokenType::Number, "num6er"),
                        (TokenType::StringLiteral, "\"string literal\""),
                        (TokenType::Punctuation, "punctuation ;"),
                        // (TokenType::Whitespace, "whitespace"),
                    ] {
                        let format = &mut self.formats[tt];
                        ui.style_mut().override_font_id = Some(format.font_id.clone());
                        ui.visuals_mut().override_text_color = Some(format.color);
                        ui.radio_value(&mut selected_tt, tt, tt_name);
                    }
                });

                let reset_value = if self.dark_mode {
                    CodeTheme::dark()
                } else {
                    CodeTheme::light()
                };

                if ui
                    .add_enabled(*self != reset_value, Button::new("Reset theme"))
                    .clicked()
                {
                    *self = reset_value;
                }
            });

            ui.add_space(16.0);

            ui.data_mut(|d| d.insert_persisted(selected_id, selected_tt));

            egui::Frame::group(ui.style())
                .inner_margin(egui::Vec2::splat(2.0))
                .show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                    ui.spacing_mut().slider_width = 128.0; // Controls color picker size
                    egui::widgets::color_picker::color_picker_color32(
                        ui,
                        &mut self.formats[selected_tt].color,
                        egui::color_picker::Alpha::Opaque,
                    );
                });
        });
    }
}

impl Highlighter {
    fn highlight(&self, theme: &CodeTheme, code: &str, errors: &ErrorsHighlightInfo) -> LayoutJob {
        self.highlight_impl(theme, code, errors)
    }
}

lazy_static! {
    static ref ASM_KEYWORDS_SET: HashSet<&'static str> = {
        let mut res = HashSet::new();
        for info in INSTRUCTION_SET {
            res.insert(info.name);
        }
        res
    };
}

pub fn wrapping_parse(mut text: &str) -> Option<u16> {
    let sign = if text.starts_with('-') {
        text = &text[1..];
        -1
    } else {
        1
    };

    let base = if text.starts_with("0x") {
        text = &text[2..];
        16
    } else if text
        .chars()
        .any(|c| c.is_ascii_hexdigit() && !c.is_ascii_digit())
    {
        16
    } else {
        10
    };
    if text.is_empty() {
        return None;
    }
    let mut res = 0i32;
    for c in text.chars().rev() {
        res = res.wrapping_mul(base as i32);
        res = res.wrapping_add(c.to_digit(base)? as i32);
    }
    Some((res * sign) as u16)
}

#[derive(Default)]
struct Highlighter {}

impl Highlighter {
    fn is_keyword(word: &str) -> bool {
        ASM_KEYWORDS_SET.contains(&word.to_ascii_lowercase().as_str())
    }

    fn highlight_impl(
        &self,
        theme: &CodeTheme,
        mut text: &str,
        errors: &ErrorsHighlightInfo,
    ) -> LayoutJob {
        let mut job = Vec::new();
        let initial_text = text;
        while !text.is_empty() {
            if text.starts_with(";") {
                let end = text.find('\n').unwrap_or(text.len());
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or(text.len());
                job.push((
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::StringLiteral].clone(),
                ));
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .map_or_else(|| text.len(), |i| i + 1);
                let word = &text[..end];
                let tt = if Self::is_keyword(word) {
                    TokenType::Keyword
                } else if wrapping_parse(word).is_some() {
                    TokenType::Number
                } else {
                    TokenType::Literal
                };
                job.push((word, 0.0, theme.formats[tt].clone()));
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map_or_else(|| text.len(), |i| i + 1);
                job.push((
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Whitespace].clone(),
                ));
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.push((
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Punctuation].clone(),
                ));
                text = &text[end..];
            }
        }
        text = initial_text;
        let mut is_error = vec![
            false;
            text.len()
                .max(errors.iter().map(|x| x.0.end).max().unwrap_or(0))
        ];
        for (error_range, _) in errors {
            for i in error_range.clone() {
                is_error[i] = true;
            }
        }
        let mut job_data: Vec<(usize, f32, TextFormat)> = Vec::new();
        let mut i = 0;
        for (text, leading_space, format) in job {
            for c in text.chars() {
                if i == 0 || is_error[i] != is_error[i - 1] || format != job_data.last().unwrap().2 {
                    job_data.push((
                        i,
                        leading_space,
                        if is_error[i] {
                            TextFormat {
                                underline: Stroke {
                                    width: 1.5,
                                    color: Color32::RED,
                                },
                                ..format.clone()
                            }
                        } else {
                            format.clone()
                        },
                    ));
                }
                i += c.len_utf8();
            }
        }
        dbg!(&errors);
        let mut job = LayoutJob::default();
        for (i, data) in job_data.iter().enumerate() {
            job.append(
                if let Some(right) = job_data.get(i + 1) {
                    &text[data.0..right.0]
                } else {
                    &text[data.0..]
                },
                data.1,
                data.clone().2,
            );
        }
        job
    }
}
