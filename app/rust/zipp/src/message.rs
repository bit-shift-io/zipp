use iced::window;
use zipp_core::gps::GpsUpdate;
use zipp_core::protocol::ScooterCommand;
use zipp_core::transport::{ScanDevice, TransportEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Noop,
    Tick,
    ReconnectTick,
    Transport(TransportEvent),
    WindowOpened(window::Id),
    BeginScan,
    StopScan,
    ScanToggle,
    Connect(ScanDevice),
    Disconnect,
    Send(ScooterCommand),
    Entry,
    EntryFinished(Result<u16, String>),
    OpenLog,
    Back,
    ClearLogs,
    UartChanged(String),
    UartSubmit,
    Gps(GpsUpdate),
    Insets(iced_android::Insets),
    SystemScale(f32),
}
