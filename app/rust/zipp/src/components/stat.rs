use iced::Element;
use iced::widget::text;

use crate::message::Message;
use crate::theme;

pub fn stat<'a>(value: impl Into<String>, size: f32) -> Element<'a, Message> {
    text(value.into()).size(size).color(theme::MUTED).into()
}

pub fn unit<'a>(value: impl Into<String>, size: f32) -> Element<'a, Message> {
    text(value.into()).size(size).color(theme::MUTED).into()
}
