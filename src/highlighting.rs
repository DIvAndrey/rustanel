use crate::compiler::ErrorsHighlightInfo;
use crate::instruction_set::INSTRUCTION_SET;
use eframe::egui;
use eframe::egui::ahash::HashSetExt;
use eframe::egui::{Color32, Stroke, TextFormat};
use eframe::epaint::ahash::HashSet;
use egui::text::LayoutJob;
use enum_map::Enum;
use lazy_static::lazy_static;

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
pub enum TokenType {
    Comment,
    Keyword,
    Literal,
    Number,
    StringLiteral,
    Punctuation,
    Whitespace,
    Label,
}

/// A selected color theme.
#[derive(Clone, Hash, PartialEq)]
pub struct CodeTheme {
    dark_mode: bool,
    pub formats: enum_map::EnumMap<TokenType, TextFormat>,
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
                TokenType::Label => TextFormat::simple(font_id.clone(), Color32::from_rgb(179, 174, 96)),
            ],
            bg_color: Color32::from_rgb(30, 31, 34),
            compiled_program: [0; 0x0100],
        }
    }

    pub fn light() -> Self {
        let font_id = egui::FontId::monospace(10.0);
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
                TokenType::Label => TextFormat::simple(font_id.clone(), Color32::from_rgb(228, 183, 34)),
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
    } else {
        10
    };
    if text.is_empty() {
        return None;
    }
    let mut res = 0i32;
    for c in text.chars() {
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
                job.push((&text[..end], 0.0, theme.formats[TokenType::Comment].clone()));
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
            } else if text.starts_with('@') {
                let end = text[1..]
                    .find(|c: char| !c.is_alphanumeric())
                    .map_or_else(|| text.len(), |i| i + 1);
                let word = &text[..end];
                job.push((word, 0.0, theme.formats[TokenType::Label].clone()));
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_alphanumeric()) {
                let mut end = text
                    .find(|c: char| !c.is_alphanumeric())
                    .unwrap_or_else(|| text.len());
                let mut word = &text[..end];
                let tt = if text[end..].chars().next() == Some(':') {
                    end += 1;
                    word = &text[..end];
                    TokenType::Label
                } else if Self::is_keyword(word) {
                    TokenType::Keyword
                } else if wrapping_parse(word).is_some() {
                    TokenType::Number
                } else {
                    TokenType::Literal
                };
                job.push((word, 0.0, theme.formats[tt].clone()));
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_whitespace())
                    .map_or_else(|| text.len(), |i| i + 1);
                job.push((
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Whitespace].clone(),
                ));
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                let _ = it.next();
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
                if i == 0 || is_error[i] != is_error[i - 1] || format != job_data.last().unwrap().2
                {
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
