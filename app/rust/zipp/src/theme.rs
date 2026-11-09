use iced::Color;

pub const BACKGROUND: Color = Color::BLACK;
pub const TEXT: Color = Color::WHITE;
pub const MUTED: Color = Color::from_rgb8(0xCC, 0xCC, 0xCC);
pub const DISABLED: Color = Color::from_rgb8(0x33, 0x33, 0x33);
pub const ACTIVE_FILL: Color = DISABLED;
pub const BORDER: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.24,
};
pub const CARD: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.10,
};
pub const WHITE_54: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.54,
};

pub const RADIUS: f32 = 12.0;
pub const BUTTON_BORDER: f32 = 2.0;
pub const DASHBOARD_HEIGHT: f32 = 480.0;
pub const ROW_HEIGHT: f32 = 90.0;

pub const SPEED_SIZE: f32 = 100.0;
pub const STAT_SIZE: f32 = 35.0;
pub const AMPM_SIZE: f32 = 25.0;
pub const BOTTOM_STAT_SIZE: f32 = 26.0;
pub const UNIT_SIZE: f32 = 24.0;
pub const BUTTON_SIZE: f32 = 20.0;
pub const TITLE_SIZE: f32 = 28.0;
pub const LIST_TITLE_SIZE: f32 = 18.0;
pub const LIST_SUBTITLE_SIZE: f32 = 14.0;
pub const LOG_SIZE: f32 = 16.0;
pub const GPS_SIZE: f32 = 12.0;
pub const STATUS_SIZE: f32 = 20.0;
pub const DEVICE_ID_SIZE: f32 = 12.0;
pub const ICON_SIZE: f32 = 32.0;
pub const SCAN_ICON_SIZE: f32 = 28.0;
pub const STOP_ICON_SIZE: f32 = 32.0;
pub const LOG_INPUT_HEIGHT: f32 = 60.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_match_the_flutter_values() {
        assert_eq!(BACKGROUND, Color::BLACK);
        assert_eq!(TEXT, Color::WHITE);
        assert_eq!(MUTED, Color::from_rgb8(0xCC, 0xCC, 0xCC));
        assert_eq!(DISABLED, Color::from_rgb8(0x33, 0x33, 0x33));
        assert!((BORDER.a - 0.24).abs() < f32::EPSILON);
        assert!((CARD.a - 0.10).abs() < f32::EPSILON);
        assert!((WHITE_54.a - 0.54).abs() < f32::EPSILON);
    }

    #[test]
    fn radii_and_metrics_are_fixed() {
        assert_eq!(RADIUS, 12.0);
        assert_eq!(BUTTON_BORDER, 2.0);
        assert_eq!(DASHBOARD_HEIGHT, 480.0);
        assert_eq!(ROW_HEIGHT, 90.0);
        assert_eq!(SPEED_SIZE, 100.0);
    }
}
