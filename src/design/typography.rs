#![allow(dead_code)]
use egui::{FontFamily, FontId, RichText};
use super::tokens::ThemeTokens;

pub const FONT_XS: f32 = 10.0;
pub const FONT_SM: f32 = 11.0;
pub const FONT_MD: f32 = 13.0;
pub const FONT_LG: f32 = 15.0;
pub const FONT_XL: f32 = 18.0;
pub const FONT_2XL: f32 = 24.0;

fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Medium".into()))
}

pub fn text_page_title(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(medium(FONT_2XL)).color(t.text_primary)
}

pub fn text_section_header(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(medium(FONT_SM)).color(t.accent)
}

pub fn text_label(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_MD)).color(t.text_primary)
}

pub fn text_body(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_MD)).color(t.text_secondary)
}

pub fn text_meta(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_SM)).color(t.text_meta)
}

pub fn text_hint(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_XS)).color(t.text_disabled)
}

pub fn text_episode_title(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_MD)).color(t.text_primary)
}

pub fn text_episode_title_played(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(regular(FONT_MD)).color(t.text_secondary)
}

pub fn text_button(text: impl Into<String>) -> RichText {
    RichText::new(text).font(regular(FONT_MD))
}

pub fn text_podcast_card_name(text: impl Into<String>, t: &ThemeTokens) -> RichText {
    RichText::new(text).font(medium(FONT_SM)).color(t.text_primary)
}
