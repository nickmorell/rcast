use egui::Color32;
use super::palette::*;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ThemeTokens {
    pub is_dark: bool,

    // Surfaces
    pub page_bg: Color32,
    pub card_bg: Color32,
    pub input_bg: Color32,
    pub hover_bg: Color32,
    pub border: Color32,
    pub divider: Color32,

    // Text
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_meta: Color32,
    pub text_disabled: Color32,

    // Brand / interactive
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_tint: Color32,

    // Episode playback state
    pub unplayed: Color32,
    pub in_progress: Color32,
    pub played: Color32,

    // Semantic
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

impl ThemeTokens {
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            page_bg: D_SURFACE_0,
            card_bg: D_SURFACE_1,
            input_bg: D_SURFACE_2,
            hover_bg: D_HOVER,
            border: D_SURFACE_3,
            divider: D_SURFACE_3,

            text_primary: D_TEXT_PRIMARY,
            text_secondary: D_TEXT_SECONDARY,
            text_meta: D_TEXT_META,
            text_disabled: D_TEXT_DISABLED,

            accent: BRAND,
            accent_hover: BRAND_DIM,
            accent_tint: BRAND_TINT_DARK,

            unplayed: BRAND,
            in_progress: IN_PROGRESS,
            played: D_PLAYED,

            success: D_SUCCESS,
            warning: D_WARNING,
            error: D_ERROR,
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            page_bg: L_SURFACE_0,
            card_bg: L_SURFACE_1,
            input_bg: L_SURFACE_2,
            hover_bg: L_HOVER,
            border: L_SURFACE_3,
            divider: L_SURFACE_3,

            text_primary: L_TEXT_PRIMARY,
            text_secondary: L_TEXT_SECONDARY,
            text_meta: L_TEXT_META,
            text_disabled: L_TEXT_DISABLED,

            accent: BRAND,
            accent_hover: BRAND_DIM,
            accent_tint: BRAND_TINT_LIGHT,

            unplayed: BRAND,
            in_progress: IN_PROG_LIGHT,
            played: L_PLAYED,

            success: L_SUCCESS,
            warning: L_WARNING,
            error: L_ERROR,
        }
    }
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::dark()
    }
}
