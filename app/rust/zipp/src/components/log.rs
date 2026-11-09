use iced::widget::{button, column, container, row, scrollable, space, stack, text, text_input};
use iced::{Background, Border, Color, Element, Length};

use crate::app::App;
use crate::components::chrome;
use crate::fonts;
use crate::icons;
use crate::message::Message;
use crate::theme;

pub const LOG_SCROLL: &str = "zipp-log-scroll";

pub fn input_style()
-> impl Fn(&iced::Theme, iced::widget::text_input::Status) -> iced::widget::text_input::Style + 'static
{
    |_theme, _status| iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: theme::MUTED,
        placeholder: theme::BORDER,
        value: theme::TEXT,
        selection: Color {
            a: 0.35,
            ..Color::WHITE
        },
    }
}

fn bold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..fonts::DEFAULT_FONT
    }
}

fn filled(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(theme::ACTIVE_FILL.into()),
        text_color: theme::TEXT,
        border: Border::default(),
        ..button::Style::default()
    }
}

pub fn terminal(app: &App) -> Element<'_, Message> {
    let strings = app.session.strings();

    let mut lines = column![].padding(10);
    for line in app.session.logs.lines() {
        lines = lines.push(
            text(line.clone())
                .size(theme::LOG_SIZE)
                .color(theme::MUTED)
                .line_height(1.2),
        );
    }
    let scroll = scrollable(lines).id(LOG_SCROLL).height(Length::Fill);

    let clear = button(text(strings.clear).size(theme::LOG_SIZE).color(theme::TEXT))
        .on_press(Message::ClearLogs)
        .padding([8, 12])
        .style(|_, _| button::Style {
            background: Some(theme::ACTIVE_FILL.into()),
            text_color: theme::TEXT,
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        });
    let clear_overlay = container(clear)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Top);

    let input = text_input("...", &app.uart_input)
        .on_input(Message::UartChanged)
        .on_submit(Message::UartSubmit)
        .size(18)
        .padding(0)
        .style(input_style());

    let send = button(chrome::icon(icons::CHEVRON_RIGHT, 30.0, theme::TEXT))
        .on_press(Message::UartSubmit)
        .padding(8)
        .style(filled);

    let input_row = container(row![input, send].align_y(iced::alignment::Vertical::Center))
        .height(Length::Fixed(theme::LOG_INPUT_HEIGHT))
        .padding([0, 10]);

    let top_border = container(space::horizontal())
        .width(Length::Fill)
        .height(1.0)
        .style(|_| container::Style {
            background: Some(theme::BORDER.into()),
            ..Default::default()
        });

    container(column![
        stack![scroll, clear_overlay].height(Length::Fill),
        top_border,
        input_row,
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::BACKGROUND.into()),
        ..Default::default()
    })
    .into()
}

pub fn page(app: &App) -> Element<'_, Message> {
    let strings = app.session.strings();

    let header = row![
        button(chrome::icon(icons::ARROW_BACK_IOS, 24.0, theme::TEXT))
            .on_press(Message::Back)
            .padding(8)
            .style(|_, _| button::Style::default()),
        text(strings.scanner_title)
            .size(theme::TITLE_SIZE)
            .color(theme::TEXT)
            .font(bold()),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center)
    .padding([12, 16]);

    container(column![header, chrome::divider(), terminal(app)])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
