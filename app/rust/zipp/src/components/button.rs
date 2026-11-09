use iced::widget::{button, column, container, text};
use iced::{Border, Element, Length};

use crate::message::Message;
use crate::theme;

pub fn fill(on: Option<bool>) -> Option<iced::Color> {
    on.filter(|value| *value).map(|_| theme::ACTIVE_FILL)
}

pub fn shows_on_off(on: Option<bool>) -> bool {
    on.is_some()
}

pub fn dashboard(
    label: impl Into<String>,
    on: Option<bool>,
    on_off: [&'static str; 2],
    message: Message,
) -> Element<'static, Message> {
    let mut body = column![
        text(label.into())
            .size(theme::BUTTON_SIZE)
            .color(theme::TEXT)
            .align_x(iced::alignment::Horizontal::Center)
    ]
    .align_x(iced::Alignment::Center)
    .spacing(0);
    if let Some(value) = on {
        body = body.push(
            text(if value { on_off[0] } else { on_off[1] })
                .size(theme::BUTTON_SIZE)
                .color(theme::TEXT),
        );
    }
    let background = fill(on);
    button(
        container(body)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(message)
    .padding(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_, _| button::Style {
        background: background.map(Into::into),
        text_color: theme::TEXT,
        border: Border {
            color: theme::BORDER,
            width: theme::BUTTON_BORDER,
            radius: theme::RADIUS.into(),
        },
        ..button::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_fill_is_present_only_when_on() {
        assert_eq!(fill(Some(true)), Some(theme::ACTIVE_FILL));
        assert_eq!(fill(Some(false)), None);
        assert_eq!(fill(None), None);
    }

    #[test]
    fn an_action_button_never_shows_on_off() {
        assert!(!shows_on_off(None));
        assert!(shows_on_off(Some(false)));
        assert!(shows_on_off(Some(true)));
    }
}
