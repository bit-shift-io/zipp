//! Zipp: an iced dashboard for the BScooter.

use iced::window;

pub mod app;
pub mod components;
pub mod fonts;
pub mod icons;
#[cfg(target_os = "android")]
mod jni;
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

/// The Android entry point installed by the native-activity glue. Logging is
/// already set up by `Java_io_bitshift_zipp_ZippBridge_nativeInit`, which runs
/// before the window starts, so a crash here is still logged.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: iced_android::AndroidApp) {
    iced_android::init(app);
    if let Err(error) = run() {
        log::error!("iced failed: {error}");
    }
}
