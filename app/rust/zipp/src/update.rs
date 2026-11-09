use std::time::Duration;

use iced::{Subscription, Task};
use zipp_core::protocol::ScooterCommand;
use zipp_core::transport::{TransportCommand, TransportEvent};

use crate::app::{App, Page};
use crate::message::Message;

fn scroll_logs() -> Task<Message> {
    iced::widget::operation::snap_to_end(crate::components::log::LOG_SCROLL)
}

impl App {
    fn send_all(&self, commands: Vec<TransportCommand>) {
        for command in commands {
            self.transport.send(command);
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop | Message::WindowOpened(_) => Task::none(),
            Message::Tick => {
                self.now = zipp_core::log::now();
                Task::none()
            }
            Message::ReconnectTick => {
                let commands = self.session.tick(self.now);
                self.send_all(commands);
                Task::none()
            }
            Message::Transport(event) => {
                let commands = self.session.apply(event.clone(), self.now);
                self.send_all(commands);
                if matches!(event, TransportEvent::Wifi(true)) && self.session.entry_pending {
                    self.session.finish_entry();
                    return Task::perform(crate::platform::send_entry(), Message::EntryFinished);
                }
                if matches!(event, TransportEvent::Wifi(_)) {
                    Task::none()
                } else {
                    scroll_logs()
                }
            }
            Message::BeginScan => {
                let commands = self.session.begin_scan();
                self.send_all(commands);
                Task::none()
            }
            Message::StopScan => {
                let commands = self.session.stop_scan();
                self.send_all(commands);
                Task::none()
            }
            Message::ScanToggle => {
                let commands = if self.session.scanning {
                    self.session.stop_scan()
                } else {
                    self.session.begin_scan()
                };
                self.send_all(commands);
                Task::none()
            }
            Message::Connect(device) => {
                let commands = self.session.connect(device, self.now);
                self.send_all(commands);
                scroll_logs()
            }
            Message::Disconnect => {
                let commands = self.session.disconnect();
                self.send_all(commands);
                Task::none()
            }
            Message::Send(command) => {
                let commands = self.session.send_command(command, self.now);
                self.send_all(commands);
                scroll_logs()
            }
            Message::Entry => {
                let mobile = self.mobile;
                let wifi = *self.transport.wifi().borrow();
                match self.session.request_entry(mobile, wifi) {
                    zipp_core::entry::EntryDecision::SendNow => {
                        Task::perform(crate::platform::send_entry(), Message::EntryFinished)
                    }
                    zipp_core::entry::EntryDecision::WaitForWifi
                    | zipp_core::entry::EntryDecision::Ignore => Task::none(),
                }
            }
            Message::EntryFinished(result) => {
                match result {
                    Ok(status) => log::info!("Entry request sent. Status: {status}"),
                    Err(error) => log::warn!("{error}"),
                }
                self.session.finish_entry();
                Task::none()
            }
            Message::OpenLog => {
                self.page = Page::Log;
                scroll_logs()
            }
            Message::Back => {
                self.page = Page::Dashboard;
                Task::none()
            }
            Message::ClearLogs => {
                self.session.logs.clear();
                Task::none()
            }
            Message::UartChanged(value) => {
                self.uart_input = value;
                Task::none()
            }
            Message::UartSubmit => {
                let value = std::mem::take(&mut self.uart_input);
                if value.is_empty() {
                    Task::none()
                } else {
                    self.update(Message::Send(ScooterCommand::SetUart(value)))
                }
            }
            Message::Gps(update) => {
                self.session.set_gps(update);
                Task::none()
            }
            Message::Insets(insets) => {
                self.insets = insets;
                Task::none()
            }
            Message::SystemScale(scale) => {
                self.scale = scale;
                Task::none()
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick),
            iced::time::every(Duration::from_secs(5)).map(|_| Message::ReconnectTick),
            crate::platform::transport_subscription(),
            crate::platform::back_subscription(),
            crate::platform::gps_subscription(),
            iced_android::insets().map(Message::Insets),
            iced_android::system_scale().map(Message::SystemScale),
            iced::keyboard::listen().filter_map(|event| match event {
                iced::keyboard::Event::KeyPressed {
                    key:
                        iced::keyboard::Key::Named(
                            iced::keyboard::key::Named::Escape
                            | iced::keyboard::key::Named::BrowserBack,
                        ),
                    ..
                } => Some(Message::Back),
                _ => None,
            }),
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::app::{App, Page};
    use crate::message::Message;
    use crate::platform::fake::FakeTransport;
    use zipp_core::protocol::ScooterCommand;
    use zipp_core::transport::{DeviceId, ScanDevice, TransportCommand, TransportEvent};

    fn app() -> (App, Arc<FakeTransport>) {
        let transport = Arc::new(FakeTransport::new());
        (App::for_test(transport.clone()), transport)
    }

    #[test]
    fn scan_toggle_starts_then_stops() {
        let (mut app, transport) = app();
        let _ = app.update(Message::ScanToggle);
        assert_eq!(
            transport.sent(),
            vec![TransportCommand::StartScan {
                target: "BScooter".into()
            }]
        );
        app.session.scanning = true;
        let _ = app.update(Message::ScanToggle);
        assert_eq!(transport.sent().last(), Some(&TransportCommand::StopScan));
    }

    #[test]
    fn reconnect_tick_only_runs_when_idle() {
        let (mut app, transport) = app();
        let _ = app.update(Message::ReconnectTick);
        assert_eq!(
            transport.sent(),
            vec![TransportCommand::Reconnect {
                target: "BScooter".into()
            }]
        );
        app.session.scanning = true;
        let _ = app.update(Message::ReconnectTick);
        assert_eq!(transport.sent().len(), 1);
    }

    #[test]
    fn scan_results_auto_connect_and_stop_the_scan() {
        let (mut app, transport) = app();
        app.session.scanning = true;
        let _ = app.update(Message::Transport(TransportEvent::Devices(vec![
            ScanDevice {
                id: DeviceId("AA".into()),
                name: "BScooter".into(),
                rssi: Some(-40),
            },
        ])));
        assert_eq!(
            transport.sent(),
            vec![
                TransportCommand::StopScan,
                TransportCommand::Connect {
                    id: DeviceId("AA".into())
                }
            ]
        );
    }

    #[test]
    fn a_connected_scooter_accepts_writes() {
        let (mut app, transport) = app();
        let _ = app.update(Message::Transport(TransportEvent::Connected {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
        }));
        let _ = app.update(Message::Send(ScooterCommand::TogglePower));
        assert!(matches!(
            transport.sent().last(),
            Some(TransportCommand::Write { bytes, .. }) if bytes == &vec![183]
        ));
    }

    #[test]
    fn back_returns_to_the_dashboard() {
        let (mut app, _) = app();
        let _ = app.update(Message::OpenLog);
        assert_eq!(app.page, Page::Log);
        let _ = app.update(Message::Back);
        assert_eq!(app.page, Page::Dashboard);
        let _ = app.update(Message::Back);
        assert_eq!(app.page, Page::Dashboard);
    }

    #[test]
    fn uart_submit_sends_the_text_and_clears_the_field() {
        let (mut app, transport) = app();
        let _ = app.update(Message::Transport(TransportEvent::Connected {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
        }));
        let _ = app.update(Message::UartChanged("hello".into()));
        let _ = app.update(Message::UartSubmit);
        assert!(app.uart_input.is_empty());
        assert!(matches!(
            transport.sent().last(),
            Some(TransportCommand::Write { bytes, .. }) if bytes == b"hello"
        ));
    }

    #[test]
    fn empty_uart_input_does_nothing() {
        let (mut app, transport) = app();
        let _ = app.update(Message::UartSubmit);
        assert!(transport.sent().is_empty());
    }

    #[test]
    fn entry_waits_for_wifi_and_sends_once() {
        let (mut app, transport) = app();
        app.mobile = true;
        transport.set_wifi(false);
        let _ = app.update(Message::Entry);
        assert!(transport.sent().is_empty());
        // The pending flag is set; a later Wifi(true) starts the HTTP task.
        assert!(app.session.entry_pending);
        let _ = app.update(Message::Transport(TransportEvent::Wifi(true)));
        assert!(!app.session.entry_pending);
    }

    #[test]
    fn clear_logs_empties_the_buffer() {
        let (mut app, _) = app();
        app.session
            .log("hello", chrono::NaiveTime::from_hms_opt(1, 2, 0).unwrap());
        let _ = app.update(Message::ClearLogs);
        assert!(app.session.logs.is_empty());
    }

    #[test]
    fn insets_and_scale_are_stored() {
        let (mut app, _) = app();
        let _ = app.update(Message::Insets(iced_android::Insets {
            top: 24.0,
            right: 1.0,
            bottom: 2.0,
            left: 3.0,
        }));
        let _ = app.update(Message::SystemScale(1.25));
        assert_eq!(app.insets.top, 24.0);
        assert!((app.scale - 1.25).abs() < f32::EPSILON);
    }
}
