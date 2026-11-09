//! Zipp: an iced dashboard for the BScooter.

use iced::window;

pub mod app;
pub mod components;
pub mod fonts;
pub mod icons;
pub mod message;
mod platform;
pub mod theme;
pub mod update;
pub mod view;

pub use app::App;
pub use message::Message;

pub fn run() -> iced::Result {
    let mut daemon = iced::daemon(App::new, App::update, view::view)
        .title(|_: &App, _: window::Id| "Zipp".into())
        .theme(|_: &App, _: window::Id| iced::Theme::Dark)
        .style(|_: &App, _: &iced::Theme| iced::theme::Style {
            background_color: theme::BACKGROUND,
            text_color: theme::TEXT,
        })
        .subscription(|app: &App| app.subscription())
        .scale_factor(|app: &App, _: window::Id| app.scale)
        .default_font(fonts::DEFAULT_FONT);

    for font in fonts::all() {
        daemon = daemon.font(font);
    }

    daemon.run()
}
