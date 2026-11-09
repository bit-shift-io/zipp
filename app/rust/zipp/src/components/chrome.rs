use iced::widget::{container, space, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::theme;

pub fn divider() -> Element<'static, Message> {
    container(space::horizontal())
        .width(Length::Fill)
        .height(theme::BUTTON_BORDER)
        .style(|_| container::Style {
            background: Some(theme::BORDER.into()),
            ..Default::default()
        })
        .into()
}

pub fn icon(character: char, size: f32, color: iced::Color) -> Element<'static, Message> {
    text(character.to_string())
        .font(crate::icons::FONT)
        .size(size)
        .color(color)
        .into()
}
