pub use crate::fonts::ICON_FONT as FONT;

pub const GPS_FIXED: char = '\u{e2dc}';
pub const GPS_NOT_FIXED: char = '\u{e2dd}';
pub const BLUETOOTH: char = '\u{e0e4}';
pub const BLUETOOTH_CONNECTED: char = '\u{e0e6}';
pub const BLUETOOTH_DISABLED: char = '\u{e0e7}';
pub const BLUETOOTH_SEARCHING: char = '\u{e0e9}';
pub const SEARCH: char = '\u{e567}';
pub const STOP: char = '\u{e606}';
pub const ARROW_BACK_IOS: char = '\u{e093}';
pub const CHEVRON_RIGHT: char = '\u{e15f}';

pub fn glyph(character: char) -> String {
    character.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codepoints_match_the_bundled_font() {
        assert_eq!(GPS_FIXED, '\u{e2dc}');
        assert_eq!(GPS_NOT_FIXED, '\u{e2dd}');
        assert_eq!(BLUETOOTH, '\u{e0e4}');
        assert_eq!(BLUETOOTH_CONNECTED, '\u{e0e6}');
        assert_eq!(BLUETOOTH_DISABLED, '\u{e0e7}');
        assert_eq!(BLUETOOTH_SEARCHING, '\u{e0e9}');
        assert_eq!(SEARCH, '\u{e567}');
        assert_eq!(STOP, '\u{e606}');
        assert_eq!(ARROW_BACK_IOS, '\u{e093}');
        assert_eq!(CHEVRON_RIGHT, '\u{e15f}');
    }
}
