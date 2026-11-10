use chrono::Timelike;
use iced::widget::{column, container, row, space, text};
use iced::{Border, Element, Length, Padding};
use zipp_core::gps::GpsStatus;
use zipp_core::protocol::{ScooterCommand, ScooterState};

use crate::app::{App, Page};
use crate::components::{button, chrome, stat};
use crate::icons;
use crate::message::Message;
use crate::theme;

pub fn value(value: Option<&str>, fallback: &str) -> String {
    match value {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => fallback.to_string(),
    }
}

pub fn clock_12h(hour: u32, minute: u32) -> (String, &'static str) {
    let pm = hour >= 12;
    let hour = if hour > 12 {
        hour - 12
    } else if hour == 0 {
        12
    } else {
        hour
    };
    (format!("{hour}:{minute:02}"), if pm { "pm" } else { "am" })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indicator {
    Brake,
    Cruise,
    Park,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Power,
    Alarm,
    Lights,
    Sport,
}

pub fn indicator(state: Option<&ScooterState>, which: Indicator) -> bool {
    match (state, which) {
        (None, _) => true,
        (Some(s), Indicator::Brake) => s.brake_active,
        (Some(s), Indicator::Cruise) => s.cruise_level > 0,
        (Some(s), Indicator::Park) => s.park_brake_active,
    }
}

pub fn toggle(state: Option<&ScooterState>, which: Toggle) -> bool {
    let Some(s) = state else {
        return false;
    };
    match which {
        Toggle::Power => s.power_on,
        Toggle::Alarm => s.alarm_on,
        Toggle::Lights => s.lights_on,
        Toggle::Sport => s.sport_on,
    }
}

pub fn speed_text(
    state: Option<&ScooterState>,
    status: GpsStatus,
    gps_speed_kmh: Option<f64>,
) -> String {
    if status == GpsStatus::Active
        && let Some(speed) = gps_speed_kmh
    {
        return format!("{:02}", speed.round() as i64);
    }
    value(state.map(|s| s.speed.as_str()), "00")
}

fn button_tile(
    label: &'static str,
    on: Option<bool>,
    on_off: [&'static str; 2],
    message: Message,
) -> Element<'static, Message> {
    container(button::dashboard(label, on, on_off, message))
        .padding(4)
        .width(Length::Fill)
        .into()
}

fn indicator_label(label: &'static str, active: bool) -> Element<'static, Message> {
    container(text(label).size(theme::BOTTOM_STAT_SIZE).color(if active {
        theme::TEXT
    } else {
        theme::DISABLED
    }))
    .height(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn gps_row(status: GpsStatus) -> Element<'static, Message> {
    let character = match status {
        GpsStatus::Active => icons::GPS_FIXED,
        GpsStatus::Searching => icons::GPS_NOT_FIXED,
        _ => return row![].into(),
    };
    row![
        chrome::icon(character, 16.0, theme::MUTED),
        text("GPS").size(theme::GPS_SIZE).color(theme::MUTED),
    ]
    .spacing(4)
    .into()
}

fn cruise_bars(cruise_level: Option<i32>) -> Element<'static, Message> {
    let mut bars = column![].height(Length::Fill);
    for level in (1..=5).rev() {
        let active = cruise_level.is_none_or(|current| current >= level);
        let color = if active {
            theme::MUTED
        } else {
            theme::DISABLED
        };
        let bar = container(space::horizontal())
            .width(20.0 + level as f32 * 15.0)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(color.into()),
                border: Border {
                    radius: 100.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });
        bars = bars.push(
            container(bar)
                .padding([4, 0])
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        );
    }
    container(bars).width(100).padding([10, 0]).into()
}

pub fn dashboard(app: &App) -> Element<'_, Message> {
    let state = app.session.state.as_ref();
    let strings = app.session.strings();
    let on_off = [strings.on, strings.off];

    let row1 = container(row![
        button_tile(
            strings.power,
            Some(toggle(state, Toggle::Power)),
            on_off,
            Message::Send(ScooterCommand::TogglePower),
        ),
        button_tile(
            strings.alarm,
            Some(toggle(state, Toggle::Alarm)),
            on_off,
            Message::Send(ScooterCommand::ToggleAlarm),
        ),
        button_tile(
            strings.lights,
            Some(toggle(state, Toggle::Lights)),
            on_off,
            Message::Send(ScooterCommand::ToggleLights),
        ),
        button_tile(strings.entry, None, on_off, Message::Entry),
    ])
    .height(theme::ROW_HEIGHT);

    let (time, ampm) = clock_12h(app.now.hour(), app.now.minute());
    let row2 = container(
        row![
            container(stat::stat(
                format!(
                    "{}{}",
                    value(state.map(|s| s.temperature.as_str()), "00"),
                    strings.celsius,
                ),
                theme::STAT_SIZE,
            ))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left),
            row![
                stat::stat(time, theme::STAT_SIZE),
                stat::stat(format!(" {ampm}"), theme::AMPM_SIZE),
            ]
            .align_y(iced::alignment::Vertical::Bottom),
            container(stat::stat(
                format!("{}%", value(state.map(|s| s.battery_level.as_str()), "0")),
                theme::STAT_SIZE,
            ))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
        ]
        .padding(Padding {
            top: 4.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        }),
    );

    let left = container(
        column![
            indicator_label(strings.brake, indicator(state, Indicator::Brake)),
            indicator_label(strings.cruise, indicator(state, Indicator::Cruise)),
            indicator_label(strings.park, indicator(state, Indicator::Park)),
        ]
        .height(Length::Fill),
    )
    .width(100);

    let speed_row = row![
        stat::stat(
            speed_text(state, app.session.gps_status, app.session.gps_speed_kmh),
            theme::SPEED_SIZE,
        ),
        stat::unit(format!(" {}", strings.kmh), theme::UNIT_SIZE),
    ]
    .align_y(iced::alignment::Vertical::Bottom);

    let throttle = container(stat::unit(
        format!(
            "{} {}",
            value(state.map(|s| s.throttle_level.as_str()), "0000"),
            strings.mv,
        ),
        theme::UNIT_SIZE,
    ))
    .padding(Padding {
        bottom: 8.0,
        ..Padding::default()
    });

    let centre = column![
        container(gps_row(app.session.gps_status)).padding(Padding {
            top: 12.0,
            ..Padding::default()
        }),
        container(speed_row)
            .height(Length::Fill)
            .center_y(Length::Fill),
        throttle,
    ]
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center);

    let right = cruise_bars(state.map(|s| s.cruise_level));

    let row3 = container(row![left, centre, right])
        .height(Length::Fill)
        .padding([0, 10]);

    let row4 = container(row![
        container(stat::stat(
            format!(
                "{} {}",
                value(state.map(|s| s.odometer.as_str()), "0000"),
                strings.km,
            ),
            theme::BOTTOM_STAT_SIZE,
        ))
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Left),
        stat::stat(
            format!(
                "{} {}",
                value(state.map(|s| s.battery_power.as_str()), "0000"),
                strings.watts,
            ),
            theme::BOTTOM_STAT_SIZE,
        ),
        container(stat::stat(
            format!(
                "{}:{}",
                value(state.map(|s| s.clock_hours.as_str()), "00"),
                value(state.map(|s| s.clock_minutes.as_str()), "00"),
            ),
            theme::BOTTOM_STAT_SIZE,
        ))
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right),
    ])
    .padding([8, 10]);

    let row5 = container(row![
        button_tile(
            strings.sport,
            Some(toggle(state, Toggle::Sport)),
            on_off,
            Message::Send(ScooterCommand::ToggleSport),
        ),
        button_tile(
            strings.cruise_down,
            None,
            on_off,
            Message::Send(ScooterCommand::CruiseDown),
        ),
        button_tile(
            strings.cruise_up,
            None,
            on_off,
            Message::Send(ScooterCommand::CruiseUp),
        ),
        button_tile(
            strings.horn,
            None,
            on_off,
            Message::Send(ScooterCommand::Horn),
        ),
    ])
    .height(theme::ROW_HEIGHT);

    container(column![
        row1,
        row2,
        chrome::divider(),
        row3,
        chrome::divider(),
        row4,
        row5,
    ])
    .height(Length::Fixed(theme::DASHBOARD_HEIGHT))
    .into()
}

fn dashboard_page(app: &App) -> Element<'_, Message> {
    column![dashboard(app), crate::components::scanner::page(app)]
        .height(Length::Fill)
        .into()
}

pub fn view(app: &App, _window: iced::window::Id) -> Element<'_, Message> {
    let body = match app.page {
        Page::Dashboard => dashboard_page(app),
        Page::Log => crate::components::log::page(app),
    };
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(app.insets)
        .style(|_| container::Style {
            background: Some(theme::BACKGROUND.into()),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use zipp_core::gps::GpsStatus;
    use zipp_core::protocol::ScooterState;

    #[test]
    fn twelve_hour_clock_matches_flutter() {
        assert_eq!(clock_12h(0, 5), ("12:05".into(), "am"));
        assert_eq!(clock_12h(11, 59), ("11:59".into(), "am"));
        assert_eq!(clock_12h(12, 0), ("12:00".into(), "pm"));
        assert_eq!(clock_12h(13, 7), ("1:07".into(), "pm"));
        assert_eq!(clock_12h(23, 59), ("11:59".into(), "pm"));
    }

    #[test]
    fn empty_values_use_the_flutter_defaults() {
        assert_eq!(value(Some(""), "00"), "00");
        assert_eq!(value(None, "00"), "00");
        assert_eq!(value(Some("42"), "00"), "42");
    }

    #[test]
    fn indicators_default_active_until_state_arrives() {
        assert!(indicator(None, Indicator::Brake));
        assert!(indicator(None, Indicator::Cruise));
        assert!(indicator(None, Indicator::Park));
        let state = ScooterState {
            brake_active: false,
            park_brake_active: false,
            ..Default::default()
        };
        assert!(!indicator(Some(&state), Indicator::Brake));
        assert!(!indicator(Some(&state), Indicator::Cruise));
        assert!(!indicator(Some(&state), Indicator::Park));
    }

    #[test]
    fn toggles_default_off() {
        assert!(!toggle(None, Toggle::Power));
        let state = ScooterState {
            power_on: true,
            ..Default::default()
        };
        assert!(toggle(Some(&state), Toggle::Power));
    }

    #[test]
    fn gps_speed_replaces_the_state_speed_only_when_active() {
        let state = ScooterState {
            speed: "12".into(),
            ..Default::default()
        };
        assert_eq!(speed_text(Some(&state), GpsStatus::Active, Some(7.4)), "07");
        assert_eq!(
            speed_text(Some(&state), GpsStatus::Searching, Some(7.4)),
            "12"
        );
        assert_eq!(speed_text(None, GpsStatus::Disabled, None), "00");
    }
}
