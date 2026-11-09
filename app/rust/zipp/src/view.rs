use iced::widget::{container, text};
use iced::{Element, Length};

use crate::app::App;
use crate::message::Message;
use crate::theme;

pub fn view(_app: &App, _window: iced::window::Id) -> Element<'_, Message> {
    container(text("Zipp").size(24).color(theme::TEXT))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BACKGROUND.into()),
            ..Default::default()
        })
        .into()
}
