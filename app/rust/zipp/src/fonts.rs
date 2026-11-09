use std::borrow::Cow;

use iced::Font;

pub const DEFAULT_FONT: Font = iced_android::DEFAULT_FONT;
pub const ICON_FONT: Font = Font::with_name("Material Icons");
pub const ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/MaterialIcons-Regular.otf");

pub fn all() -> Vec<Cow<'static, [u8]>> {
    let mut fonts = iced_android::fonts();
    if let Some(cjk) = desktop_cjk_font() {
        fonts.push(Cow::Owned(cjk));
    }
    fonts.push(Cow::Borrowed(ICON_FONT_BYTES));
    fonts
}

#[cfg(target_os = "linux")]
fn desktop_cjk_font() -> Option<Vec<u8>> {
    const PATHS: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
    ];
    PATHS.iter().find_map(|path| std::fs::read(path).ok())
}

#[cfg(not(target_os = "linux"))]
fn desktop_cjk_font() -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_font_is_registered_last() {
        let fonts = all();
        assert_eq!(
            fonts.last().map(|font| font.as_ref()),
            Some(ICON_FONT_BYTES)
        );
    }

    #[test]
    fn the_bundled_font_is_an_open_type_file() {
        assert_eq!(&ICON_FONT_BYTES[..4], b"OTTO");
        assert_eq!(ICON_FONT.family, iced::font::Family::Name("Material Icons"));
    }
}
