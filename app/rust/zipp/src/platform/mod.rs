use std::sync::{Arc, OnceLock};

use futures::SinkExt;
use iced::Subscription;
use tokio::sync::{broadcast, watch};
use zipp_core::i18n::Lang;
use zipp_core::transport::{Transport, TransportCommand, TransportEvent};

use crate::message::Message;

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

pub async fn send_entry() -> Result<u16, String> {
    let response = reqwest::get(zipp_core::entry::ENTRY_URL)
        .await
        .map_err(|error| format!("Error sending entry request: {error}"))?;
    Ok(response.status().as_u16())
}

pub fn transport_subscription() -> Subscription<Message> {
    Subscription::run(transport_stream)
}

fn transport_stream() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(64, async move |mut output| {
        let mut receiver = transport().subscribe();
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if output.send(Message::Transport(event)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::Mutex;

    use tokio::sync::{broadcast, watch};
    use zipp_core::transport::{Transport, TransportCommand, TransportEvent};

    pub struct FakeTransport {
        events: broadcast::Sender<TransportEvent>,
        wifi: watch::Sender<bool>,
        sent: Mutex<Vec<TransportCommand>>,
    }

    impl FakeTransport {
        pub fn new() -> Self {
            let (events, _) = broadcast::channel(64);
            let (wifi, _) = watch::channel(true);
            Self {
                events,
                wifi,
                sent: Mutex::new(Vec::new()),
            }
        }

        #[allow(dead_code)]
        pub fn emit(&self, event: TransportEvent) {
            let _ = self.events.send(event);
        }

        pub fn sent(&self) -> Vec<TransportCommand> {
            self.sent.lock().unwrap().clone()
        }

        pub fn set_wifi(&self, value: bool) {
            // `send` fails without a live receiver, and `new` drops the one
            // from `watch::channel`; `send_replace` always stores the value.
            self.wifi.send_replace(value);
        }
    }

    impl Transport for FakeTransport {
        fn send(&self, command: TransportCommand) {
            self.sent.lock().unwrap().push(command);
        }
        fn subscribe(&self) -> broadcast::Receiver<TransportEvent> {
            self.events.subscribe()
        }
        fn wifi(&self) -> watch::Receiver<bool> {
            self.wifi.subscribe()
        }
    }
}
