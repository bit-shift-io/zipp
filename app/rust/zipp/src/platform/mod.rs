use std::sync::{Arc, OnceLock};

use futures::SinkExt;
use iced::Subscription;
#[cfg(not(target_os = "linux"))]
use tokio::sync::{broadcast, watch};
use zipp_core::i18n::Lang;
use zipp_core::transport::Transport;
#[cfg(not(target_os = "linux"))]
use zipp_core::transport::{TransportCommand, TransportEvent};

use crate::message::Message;

pub(crate) mod android;
#[cfg(target_os = "linux")]
mod desktop;

/// Stand-in transport for non-Linux targets and tests that only need the
/// channel plumbing. Linux uses the real `btleplug` actor instead.
#[cfg(not(target_os = "linux"))]
pub struct NoopTransport {
    events: broadcast::Sender<TransportEvent>,
    wifi: watch::Sender<bool>,
}

#[cfg(not(target_os = "linux"))]
impl NoopTransport {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        let (wifi, _) = watch::channel(true);
        Self { events, wifi }
    }
}

#[cfg(not(target_os = "linux"))]
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

/// The single initialiser shared by [`ensure_transport`] and [`transport`], so
/// both observe the same real transport on Linux instead of one falling back to
/// the no-op.
fn init_transport() -> Arc<dyn Transport> {
    #[cfg(target_os = "android")]
    {
        android::transport()
            .map(|transport| transport as Arc<dyn Transport>)
            .unwrap_or_else(|| {
                log::warn!(
                    "Android transport was not installed before the app started; \
                     falling back to NoopTransport"
                );
                Arc::new(NoopTransport::new())
            })
    }
    #[cfg(target_os = "linux")]
    {
        desktop::spawn()
    }
    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    {
        Arc::new(NoopTransport::new())
    }
}

pub fn ensure_transport() {
    TRANSPORT.get_or_init(init_transport);
}

pub fn transport() -> Arc<dyn Transport> {
    TRANSPORT.get_or_init(init_transport).clone()
}

pub fn lang() -> Lang {
    #[cfg(target_os = "android")]
    {
        android::current_lang()
    }
    #[cfg(not(target_os = "android"))]
    {
        zipp_core::i18n::detect()
    }
}

/// The Android hardware Back button arrives on a broadcast channel; everywhere
/// else the keyboard path already drives `Message::Back`.
pub fn back_subscription() -> Subscription<Message> {
    #[cfg(target_os = "android")]
    {
        android::back_subscription()
    }
    #[cfg(not(target_os = "android"))]
    {
        Subscription::none()
    }
}

/// GPS updates bypass `TransportEvent` on a dedicated channel so they reach
/// `Message::Gps` without round-tripping through the session.
pub fn gps_subscription() -> Subscription<Message> {
    #[cfg(target_os = "android")]
    {
        android::gps_subscription()
    }
    #[cfg(not(target_os = "android"))]
    {
        Subscription::none()
    }
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
