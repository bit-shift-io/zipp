use std::sync::{Arc, OnceLock};

use tokio::sync::{broadcast, watch};
use zipp_core::i18n::Lang;
use zipp_core::transport::{Transport, TransportCommand, TransportEvent};

pub struct NoopTransport {
    events: broadcast::Sender<TransportEvent>,
    wifi: watch::Sender<bool>,
}

impl NoopTransport {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        let (wifi, _) = watch::channel(true);
        Self { events, wifi }
    }
}

impl Transport for NoopTransport {
    fn send(&self, _command: TransportCommand) {}
    fn subscribe(&self) -> broadcast::Receiver<TransportEvent> {
        self.events.subscribe()
    }
    fn wifi(&self) -> watch::Receiver<bool> {
        self.wifi.subscribe()
    }
}

static TRANSPORT: OnceLock<Arc<dyn Transport>> = OnceLock::new();

pub fn ensure_transport() {
    TRANSPORT.get_or_init(|| Arc::new(NoopTransport::new()));
}

pub fn transport() -> Arc<dyn Transport> {
    TRANSPORT
        .get_or_init(|| Arc::new(NoopTransport::new()))
        .clone()
}

pub fn lang() -> Lang {
    zipp_core::i18n::detect()
}
