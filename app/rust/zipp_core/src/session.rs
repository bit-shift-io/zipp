//! Pure scooter session: the state machine that turns transport events into
//! transport commands. No I/O, no platform code, and time is passed in.

use chrono::NaiveTime;

use crate::entry::{self, EntryDecision};
use crate::gps::{GpsStatus, GpsUpdate};
use crate::i18n::{Lang, Strings};
use crate::log::LogBuffer;
use crate::protocol::{
    BleAction, ScooterCommand, ScooterState, get_command_action, parse_characteristic_data,
    target_device_name,
};
use crate::transport::{DeviceId, ScanDevice, TransportCommand, TransportEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDevice {
    pub id: DeviceId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ScooterSession {
    pub state: Option<ScooterState>,
    pub devices: Vec<ScanDevice>,
    pub connected: Option<ConnectedDevice>,
    pub scanning: bool,
    pub connecting: bool,
    /// Optimistic `true` until the platform reports otherwise, so the first
    /// scan is not suppressed before `AdapterState(true)` arrives.
    pub adapter_on: bool,
    pub logs: LogBuffer,
    pub gps_status: GpsStatus,
    pub gps_speed_kmh: Option<f64>,
    pub entry_pending: bool,
    pub lang: Lang,
    pub target: String,
}

impl ScooterSession {
    pub fn new(lang: Lang) -> Self {
        Self {
            state: None,
            devices: Vec::new(),
            connected: None,
            scanning: false,
            connecting: false,
            adapter_on: true,
            logs: LogBuffer::new(),
            gps_status: GpsStatus::Disabled,
            gps_speed_kmh: None,
            entry_pending: false,
            lang,
            target: target_device_name(),
        }
    }

    pub fn strings(&self) -> &'static Strings {
        self.lang.strings()
    }

    pub fn log(&mut self, message: impl AsRef<str>, now: NaiveTime) {
        self.logs.push(message, now);
    }

    pub fn begin_scan(&mut self) -> Vec<TransportCommand> {
        if !self.adapter_on {
            return Vec::new();
        }
        vec![TransportCommand::StartScan {
            target: self.target.clone(),
        }]
    }

    pub fn stop_scan(&mut self) -> Vec<TransportCommand> {
        vec![TransportCommand::StopScan]
    }

    pub fn connect(&mut self, device: ScanDevice, now: NaiveTime) -> Vec<TransportCommand> {
        if self.connecting {
            return Vec::new();
        }
        let mut commands = Vec::new();
        if self.scanning {
            self.scanning = false;
            self.log("stop scan", now);
            commands.push(TransportCommand::StopScan);
        }
        self.connecting = true;
        self.log(format!("connecting: {}", device.name), now);
        commands.push(TransportCommand::Connect { id: device.id });
        commands
    }

    pub fn disconnect(&mut self) -> Vec<TransportCommand> {
        vec![TransportCommand::Disconnect]
    }

    pub fn send_command(
        &mut self,
        command: ScooterCommand,
        now: NaiveTime,
    ) -> Vec<TransportCommand> {
        if matches!(command, ScooterCommand::SetUart(_)) {
            self.log("", now);
        }
        match &command {
            ScooterCommand::SetUart(text) => self.log(format!("> {text}"), now),
            other => self.log(format!("> sending command: {other}"), now),
        }
        if self.connected.is_none() {
            self.log("Not connected", now);
            return Vec::new();
        }
        let state = self.state.clone().unwrap_or_default();
        let BleAction {
            bytes,
            service_uuid,
            characteristic_uuid,
        } = get_command_action(&command, &state);
        vec![TransportCommand::Write {
            service: service_uuid,
            characteristic: characteristic_uuid,
            bytes,
        }]
    }

    pub fn tick(&mut self, _now: NaiveTime) -> Vec<TransportCommand> {
        if self.connected.is_none() && !self.connecting && !self.scanning {
            vec![TransportCommand::Reconnect {
                target: self.target.clone(),
            }]
        } else {
            Vec::new()
        }
    }

    pub fn apply(&mut self, event: TransportEvent, now: NaiveTime) -> Vec<TransportCommand> {
        match event {
            TransportEvent::AdapterState(on) => {
                self.adapter_on = on;
                if !on {
                    self.scanning = false;
                    self.log("Bluetooth is off", now);
                }
                Vec::new()
            }
            TransportEvent::ScanState(scanning) => {
                if scanning && !self.scanning {
                    self.devices.clear();
                    self.log("start scan", now);
                }
                self.scanning = scanning;
                Vec::new()
            }
            TransportEvent::Devices(devices) => {
                self.devices = devices;
                if self.connected.is_none()
                    && !self.connecting
                    && let Some(device) =
                        self.devices.iter().find(|d| d.name == self.target).cloned()
                {
                    return self.connect(device, now);
                }
                Vec::new()
            }
            TransportEvent::Connected { id, name } => {
                self.connecting = false;
                self.scanning = false;
                self.log(format!("connected: {name}"), now);
                self.connected = Some(ConnectedDevice { id, name });
                Vec::new()
            }
            TransportEvent::Disconnected => {
                self.connecting = false;
                self.scanning = false;
                self.connected = None;
                self.log("disconnected from device", now);
                Vec::new()
            }
            TransportEvent::Notification { uuid, data } => {
                let state = self.state.clone().unwrap_or_default();
                let parsed = parse_characteristic_data(state, &uuid, &data);
                if let Some(line) = parsed.log {
                    self.log(line, now);
                }
                self.state = Some(parsed.state);
                Vec::new()
            }
            TransportEvent::ReadValue { uuid, data } => {
                let state = self.state.clone().unwrap_or_default();
                self.state = Some(parse_characteristic_data(state, &uuid, &data).state);
                Vec::new()
            }
            TransportEvent::Wifi(_) => Vec::new(),
            TransportEvent::Error(message) => {
                if self.connecting {
                    self.connecting = false;
                }
                self.log(message, now);
                Vec::new()
            }
        }
    }

    pub fn set_gps(&mut self, update: GpsUpdate) {
        self.gps_status = update.status;
        if let Some(kmh) = update.speed_kmh() {
            self.gps_speed_kmh = Some(kmh);
        }
    }

    pub fn request_entry(&mut self, mobile: bool, wifi: bool) -> EntryDecision {
        let decision = entry::decide(mobile, wifi, self.entry_pending);
        if decision == EntryDecision::WaitForWifi {
            self.entry_pending = true;
        }
        decision
    }

    pub fn finish_entry(&mut self) {
        self.entry_pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn now() -> NaiveTime {
        NaiveTime::from_hms_opt(9, 5, 0).unwrap()
    }

    fn device(name: &str) -> ScanDevice {
        ScanDevice {
            id: DeviceId(name.into()),
            name: name.into(),
            rssi: Some(-40),
        }
    }

    #[test]
    fn starts_disconnected_and_without_a_state() {
        let s = ScooterSession::new(Lang::En);
        assert!(s.state.is_none());
        assert!(s.connected.is_none());
        assert!(!s.scanning);
        assert!(!s.connecting);
        assert!(s.adapter_on);
        assert!(s.logs.is_empty());
    }

    #[test]
    fn scan_state_logs_and_clears_results() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::Devices(vec![device("Other")]), now());
        assert_eq!(s.devices.len(), 1);
        s.apply(TransportEvent::ScanState(true), now());
        assert!(s.scanning);
        assert!(s.devices.is_empty());
        assert_eq!(s.logs.lines().iter().next().unwrap(), "09:05 start scan");
        s.apply(TransportEvent::ScanState(false), now());
        assert!(!s.scanning);
    }

    #[test]
    fn auto_connects_to_the_target_once() {
        let mut s = ScooterSession::new(Lang::En);
        let commands = s.apply(
            TransportEvent::Devices(vec![device("Other"), device("BScooter")]),
            now(),
        );
        assert_eq!(
            commands,
            vec![TransportCommand::Connect {
                id: DeviceId("BScooter".into())
            }]
        );
        assert!(s.connecting);
        assert_eq!(
            s.logs.lines().iter().next_back().unwrap(),
            "09:05 connecting: BScooter"
        );
        let second = s.apply(TransportEvent::Devices(vec![device("BScooter")]), now());
        assert!(second.is_empty());
    }

    #[test]
    fn connecting_from_the_list_stops_scanning_first() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::ScanState(true), now());
        let commands = s.connect(device("BScooter"), now());
        assert_eq!(
            commands,
            vec![
                TransportCommand::StopScan,
                TransportCommand::Connect {
                    id: DeviceId("BScooter".into())
                }
            ]
        );
        assert!(!s.scanning);
        assert!(s.connecting);
    }

    #[test]
    fn connected_and_disconnected_update_the_state_and_log() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Connected {
                id: DeviceId("BScooter".into()),
                name: "BScooter".into(),
            },
            now(),
        );
        assert!(!s.connecting);
        assert_eq!(s.connected.as_ref().unwrap().name, "BScooter");
        assert_eq!(
            s.logs.lines().iter().next_back().unwrap(),
            "09:05 connected: BScooter"
        );
        s.apply(TransportEvent::Disconnected, now());
        assert!(s.connected.is_none());
        assert_eq!(
            s.logs.lines().iter().next_back().unwrap(),
            "09:05 disconnected from device"
        );
    }

    #[test]
    fn notifications_parse_uart_into_the_log_and_state() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Notification {
                uuid: "2001".into(),
                data: vec![12],
            },
            now(),
        );
        assert_eq!(s.state.as_ref().unwrap().speed, "12");
        s.apply(
            TransportEvent::Notification {
                uuid: "0003".into(),
                data: b"hello".to_vec(),
            },
            now(),
        );
        assert_eq!(s.logs.lines().iter().next_back().unwrap(), "09:05 hello");
    }

    #[test]
    fn reads_update_state_without_logging() {
        let mut s = ScooterSession::new(Lang::En);
        let before = s.logs.len();
        s.apply(
            TransportEvent::ReadValue {
                uuid: "2a19".into(),
                data: vec![77],
            },
            now(),
        );
        assert_eq!(s.state.as_ref().unwrap().battery_level, "77");
        assert_eq!(s.logs.len(), before);
    }

    #[test]
    fn adapter_off_logs_and_stops_scanning() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::ScanState(true), now());
        s.apply(TransportEvent::AdapterState(false), now());
        assert!(!s.adapter_on);
        assert!(!s.scanning);
        assert_eq!(
            s.logs.lines().iter().next_back().unwrap(),
            "09:05 Bluetooth is off"
        );
    }

    #[test]
    fn tick_reconnects_only_when_idle() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(
            s.tick(now()),
            vec![TransportCommand::Reconnect {
                target: "BScooter".into()
            }]
        );
        s.apply(TransportEvent::ScanState(true), now());
        assert!(s.tick(now()).is_empty());
        s.apply(TransportEvent::ScanState(false), now());
        s.apply(
            TransportEvent::Connected {
                id: DeviceId("BScooter".into()),
                name: "BScooter".into(),
            },
            now(),
        );
        assert!(s.tick(now()).is_empty());
    }

    #[test]
    fn commands_log_the_legacy_text_and_write_the_action() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Connected {
                id: DeviceId("BScooter".into()),
                name: "BScooter".into(),
            },
            now(),
        );
        s.logs.clear();
        let commands = s.send_command(ScooterCommand::TogglePower, now());
        assert_eq!(
            s.logs.lines().iter().next().unwrap(),
            "09:05 > sending command: TogglePower"
        );
        assert!(matches!(
            commands.as_slice(),
            [TransportCommand::Write { service, characteristic, bytes }]
                if service == "00001000-0000-1000-8000-00805F9B34FB"
                    && characteristic == "00001001-0000-1000-8000-00805F9B34FB"
                    && bytes == &vec![183]
        ));
    }

    #[test]
    fn uart_commands_add_an_empty_line_then_the_text() {
        let mut s = ScooterSession::new(Lang::En);
        let commands = s.send_command(ScooterCommand::SetUart("abc".into()), now());
        assert!(commands.is_empty());
        assert_eq!(s.logs.lines()[0], "09:05 ");
        assert_eq!(s.logs.lines()[1], "09:05 > abc");
        assert_eq!(s.logs.lines()[2], "09:05 Not connected");
    }

    #[test]
    fn entry_waits_only_on_mobile_without_wifi() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(s.request_entry(false, false), EntryDecision::SendNow);
        assert_eq!(s.request_entry(true, true), EntryDecision::SendNow);
        assert_eq!(s.request_entry(true, false), EntryDecision::WaitForWifi);
        assert!(s.entry_pending);
        assert_eq!(s.request_entry(true, true), EntryDecision::Ignore);
        s.finish_entry();
        assert!(!s.entry_pending);
    }

    #[test]
    fn gps_updates_store_kmh() {
        let mut s = ScooterSession::new(Lang::En);
        s.set_gps(GpsUpdate {
            status: GpsStatus::Active,
            speed_mps: Some(2.5),
        });
        assert_eq!(s.gps_status, GpsStatus::Active);
        assert!((s.gps_speed_kmh.unwrap() - 9.0).abs() < 1e-9);
    }
}
