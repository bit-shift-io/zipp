use std::sync::Arc;

use chrono::NaiveTime;
use iced::window;
use iced::{Size, Task};
use zipp_core::i18n::Lang;
use zipp_core::session::ScooterSession;
use zipp_core::transport::Transport;

use crate::message::Message;
use crate::platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Log,
}

pub struct App {
    pub session: ScooterSession,
    pub transport: Arc<dyn Transport>,
    pub page: Page,
    pub uart_input: String,
    pub insets: iced_android::Insets,
    pub scale: f32,
    pub now: NaiveTime,
    pub mobile: bool,
}

pub fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(420.0, 820.0),
        min_size: Some(Size::new(360.0, 640.0)),
        resizable: true,
        ..Default::default()
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        platform::ensure_transport();
        let (_id, open) = window::open(window_settings());
        iced_android::set_system_bars_dark(true);
        let lang = platform::lang();
        let app = Self::with_transport(platform::transport(), lang);
        (app, open.map(|_| Message::Noop))
    }

    pub fn with_transport(transport: Arc<dyn Transport>, lang: Lang) -> Self {
        Self {
            session: ScooterSession::new(lang),
            transport,
            page: Page::Dashboard,
            uart_input: String::new(),
            insets: iced_android::Insets::default(),
            scale: 1.0,
            now: zipp_core::log::now(),
            mobile: cfg!(target_os = "android"),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(transport: Arc<dyn Transport>) -> Self {
        Self::with_transport(transport, Lang::En)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_one_resizable_window_at_the_legacy_size() {
        let settings = window_settings();
        assert_eq!(settings.size, Size::new(420.0, 820.0));
        assert_eq!(settings.min_size, Some(Size::new(360.0, 640.0)));
        assert!(settings.resizable);
    }
}
