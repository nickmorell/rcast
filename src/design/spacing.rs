use egui::CornerRadius;

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 24.0;
pub const SPACE_6: f32 = 32.0;
pub const SPACE_7: f32 = 48.0;

#[allow(dead_code)]
pub const PAGE_MARGIN: f32 = SPACE_6;
pub const CARD_PADDING: f32 = SPACE_4;
#[allow(dead_code)]
pub const ROW_PAD_V: f32 = SPACE_3;
#[allow(dead_code)]
pub const ROW_PAD_H: f32 = SPACE_4;
pub const SECTION_GAP: f32 = SPACE_5;
pub const CONTROL_GAP: f32 = SPACE_2;
pub const ICON_GAP: f32 = SPACE_1;

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_LG: f32 = 12.0;

pub fn rounding_sm() -> CornerRadius {
    CornerRadius::same(RADIUS_SM as u8)
}
#[allow(dead_code)]
pub fn rounding_md() -> CornerRadius {
    CornerRadius::same(RADIUS_MD as u8)
}
pub fn rounding_lg() -> CornerRadius {
    CornerRadius::same(RADIUS_LG as u8)
}
#[allow(dead_code)]
pub fn rounding_pill() -> CornerRadius {
    CornerRadius::same(u8::MAX)
}
