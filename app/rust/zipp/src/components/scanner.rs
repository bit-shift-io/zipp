use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Border, Element, Length, Padding};
use zipp_core::session::ScooterSession;

use crate::app::App;
use crate::components::chrome;
use crate::fonts;
use crate::icons;
use crate::message::Message;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Connected,
    Connecting,
    Scanning,
    Disconnected,
}

pub fn status(session: &ScooterSession) -> Status {
    if session.connected.is_some() {
        Status::Connected
    } else if session.connecting {
        Status::Connecting
    } else if session.scanning {
        Status::Scanning
    } else {
        Status::Disconnected
    }
}

pub fn scan_message(scanning: bool) -> Message {
    if scanning {
        Message::StopScan
    } else {
        Message::BeginScan
    }
}

fn bold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..fonts::DEFAULT_FONT
    }
}

/// A button that only makes its contents clickable: no background and no
/// border, matching the transparent `InkWell`/`ListTile` look in the original app.
fn transparent(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style::default()
}

fn left_padding() -> Padding {
    Padding {
        left: 16.0,
        ..Padding::default()
    }
}

fn centre() -> iced::alignment::Vertical {
    iced::alignment::Vertical::Center
}

fn status_row(app: &App) -> Element<'_, Message> {
    let session = &app.session;
    let strings = session.strings();
    let state = status(session);

    let icon = match state {
        Status::Connected => icons::BLUETOOTH_CONNECTED,
        Status::Connecting | Status::Scanning => icons::BLUETOOTH_SEARCHING,
        Status::Disconnected => icons::BLUETOOTH_DISABLED,
    };

    let label = match state {
        Status::Connected => session
            .connected
            .as_ref()
            .map(|device| device.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| strings.unknown.to_string()),
        Status::Connecting => strings.connecting.to_string(),
        Status::Scanning => strings.scanning.to_string(),
        Status::Disconnected => strings.disconnected.to_string(),
    };

    let mut details = column![
        text(label)
            .size(theme::STATUS_SIZE)
            .color(theme::MUTED)
            .font(bold())
    ];
    if let Some(device) = session.connected.as_ref() {
        details = details.push(
            text(device.id.0.clone())
                .size(theme::DEVICE_ID_SIZE)
                .color(theme::WHITE_54),
        );
    }

    let info = button(
        container(
            row![
                chrome::icon(icon, theme::ICON_SIZE, theme::MUTED),
                container(details)
                    .width(Length::Fill)
                    .padding(left_padding()),
            ]
            .align_y(centre()),
        )
        .width(Length::Fill),
    )
    .on_press(Message::OpenLog)
    .width(Length::Fill)
    .padding(0)
    .style(transparent);

    let scan_icon = if session.scanning {
        chrome::icon(icons::STOP, theme::STOP_ICON_SIZE, theme::MUTED)
    } else {
        chrome::icon(icons::SEARCH, theme::SCAN_ICON_SIZE, theme::MUTED)
    };
    let scan = button(container(scan_icon).center_x(Length::Fill))
        .on_press(scan_message(session.scanning))
        .padding(8)
        .style(transparent);

    container(row![info, scan].align_y(centre()))
        .width(Length::Fill)
        .padding([16, 24])
        .into()
}

fn scanner_list(app: &App) -> Element<'_, Message> {
    let session = &app.session;
    let strings = session.strings();
    let mut items = column![].padding([0, 8]);

    if let Some(connected) = session.connected.as_ref() {
        let name = if connected.name.is_empty() {
            strings.unknown.to_string()
        } else {
            connected.name.clone()
        };
        let disconnect = button(
            text(strings.disconnect)
                .size(theme::LIST_SUBTITLE_SIZE)
                .color(theme::MUTED),
        )
        .on_press(Message::Disconnect)
        .padding([4, 8])
        .style(transparent);

        items = items.push(
            container(
                row![
                    chrome::icon(icons::BLUETOOTH_CONNECTED, 24.0, theme::MUTED),
                    container(text(name).size(theme::LIST_TITLE_SIZE).color(theme::TEXT))
                        .width(Length::Fill)
                        .padding(left_padding()),
                    disconnect,
                ]
                .align_y(centre()),
            )
            .padding(12)
            .style(|_| container::Style {
                background: Some(theme::CARD.into()),
                border: Border {
                    radius: theme::RADIUS.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
        );
    }

    for device in &session.devices {
        let is_connected = session
            .connected
            .as_ref()
            .is_some_and(|connected| connected.id == device.id);
        if is_connected {
            continue;
        }
        let name = if device.name.is_empty() {
            strings.unknown.to_string()
        } else {
            device.name.clone()
        };
        items = items.push(
            button(
                row![
                    chrome::icon(icons::BLUETOOTH, 24.0, theme::TEXT),
                    container(
                        column![
                            text(name).size(theme::LIST_TITLE_SIZE).color(theme::TEXT),
                            text(device.id.0.clone())
                                .size(theme::LIST_SUBTITLE_SIZE)
                                .color(theme::MUTED),
                        ]
                        .spacing(0),
                    )
                    .width(Length::Fill)
                    .padding(left_padding()),
                ]
                .align_y(centre()),
            )
            .on_press(Message::Connect(device.clone()))
            .width(Length::Fill)
            .padding(12)
            .style(transparent),
        );
    }

    if session.devices.is_empty() && !session.scanning && session.connected.is_none() {
        items = items.push(
            container(
                text("No devices found")
                    .size(theme::LOG_SIZE)
                    .color(theme::GREY),
            )
            .padding(16)
            .width(Length::Fill)
            .center_x(Length::Fill),
        );
    }

    scrollable(items).height(Length::Fill).into()
}

pub fn page(app: &App) -> Element<'_, Message> {
    container(column![status_row(app), chrome::divider(), scanner_list(app)].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BACKGROUND.into()),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use zipp_core::i18n::Lang;
    use zipp_core::session::ScooterSession;

    #[test]
    fn status_prefers_connected_then_connecting_then_scanning() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(status(&s), Status::Disconnected);
        s.scanning = true;
        assert_eq!(status(&s), Status::Scanning);
        s.connecting = true;
        assert_eq!(status(&s), Status::Connecting);
        s.connected = Some(zipp_core::session::ConnectedDevice {
            id: zipp_core::transport::DeviceId("AA".into()),
            name: "BScooter".into(),
        });
        assert_eq!(status(&s), Status::Connected);
    }

    #[test]
    fn the_scan_button_stops_an_active_scan() {
        assert_eq!(scan_message(true), crate::message::Message::StopScan);
        assert_eq!(scan_message(false), crate::message::Message::BeginScan);
    }
}
