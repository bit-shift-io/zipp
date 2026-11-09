use iced::window;
use zipp_core::transport::TransportEvent;

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    Tick,
    Transport(TransportEvent),
    WindowOpened(window::Id),
}
