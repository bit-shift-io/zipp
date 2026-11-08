use tokio::sync::{broadcast, watch};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDevice {
    pub id: DeviceId,
    pub name: String,
    pub rssi: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportCommand {
    /// Android checks bonded devices first, then scans. Desktop scans.
    Reconnect {
        target: String,
    },
    StartScan {
        target: String,
    },
    StopScan,
    Connect {
        id: DeviceId,
    },
    Disconnect,
    Write {
        service: String,
        characteristic: String,
        bytes: Vec<u8>,
    },
    Read {
        service: String,
        characteristic: String,
    },
    RequestPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportEvent {
    AdapterState(bool),
    ScanState(bool),
    Devices(Vec<ScanDevice>),
    Connected { id: DeviceId, name: String },
    Disconnected,
    Notification { uuid: String, data: Vec<u8> },
    ReadValue { uuid: String, data: Vec<u8> },
    Wifi(bool),
    Error(String),
}

pub trait Transport: Send + Sync + 'static {
    fn send(&self, command: TransportCommand);
    fn subscribe(&self) -> broadcast::Receiver<TransportEvent>;
    fn wifi(&self) -> watch::Receiver<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_devices_are_ordered_and_comparable() {
        let a = ScanDevice {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
            rssi: Some(-40),
        };
        let b = ScanDevice {
            id: DeviceId("BB".into()),
            name: "Other".into(),
            rssi: None,
        };
        assert_ne!(a, b);
        assert_eq!(a.id.0, "AA");
    }

    #[test]
    fn commands_and_events_round_trip_their_payloads() {
        let command = TransportCommand::Write {
            service: "s".into(),
            characteristic: "c".into(),
            bytes: vec![1, 2],
        };
        assert!(matches!(command, TransportCommand::Write { bytes, .. } if bytes == vec![1, 2]));
        let event = TransportEvent::Connected {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
        };
        assert!(matches!(event, TransportEvent::Connected { name, .. } if name == "BScooter"));
    }
}
