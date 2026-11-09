//! Desktop BLE transport backed by `btleplug` (BlueZ on Linux).
//!
//! A dedicated thread owns a current-thread tokio runtime. Commands arrive on
//! an unbounded channel; events leave on a broadcast channel. The actor never
//! panics when there is no adapter: it reports `AdapterState(false)` and keeps
//! serving commands, retrying adapter creation on the next scan or reconnect.

use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{
    Central, CentralState, CharPropFlags, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;
use zipp_core::transport::{DeviceId, ScanDevice, Transport, TransportCommand, TransportEvent};

/// How long a single scan window runs before it stops on its own.
const SCAN_WINDOW: Duration = Duration::from_secs(15);
/// How often the peripheral snapshot is refreshed while scanning.
const SCAN_TICK: Duration = Duration::from_millis(500);
/// How long a `connect()` may take before it is reported as timed out.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct DesktopTransport {
    commands: mpsc::UnboundedSender<TransportCommand>,
    events: broadcast::Sender<TransportEvent>,
    wifi: watch::Sender<bool>,
}

impl Transport for DesktopTransport {
    fn send(&self, command: TransportCommand) {
        let _ = self.commands.send(command);
    }

    fn subscribe(&self) -> broadcast::Receiver<TransportEvent> {
        self.events.subscribe()
    }

    fn wifi(&self) -> watch::Receiver<bool> {
        self.wifi.subscribe()
    }
}

/// Starts the desktop BLE actor on its own thread and returns its handle.
pub fn spawn() -> Arc<dyn Transport> {
    let (events, _) = broadcast::channel(256);
    let (wifi, _) = watch::channel(true);
    let (commands, receiver) = mpsc::unbounded_channel();
    let transport = Arc::new(DesktopTransport {
        commands,
        events: events.clone(),
        wifi: wifi.clone(),
    });
    std::thread::Builder::new()
        .name("zipp-bluetooth".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("bluetooth runtime");
            runtime.block_on(actor(receiver, events, wifi));
        })
        .expect("spawn bluetooth thread");
    transport
}

/// Chooses the write kind a characteristic property allows.
pub fn write_type(without_response: bool) -> WriteType {
    if without_response {
        WriteType::WithoutResponse
    } else {
        WriteType::WithResponse
    }
}

/// Prefers the advertised local name, falling back to the peripheral id.
pub fn device_name(local: Option<&str>, id: &str) -> String {
    local
        .filter(|name| !name.is_empty())
        .unwrap_or(id)
        .to_string()
}

struct Actor {
    adapter: Option<Adapter>,
    peripherals: Vec<Peripheral>,
    connected: Option<Peripheral>,
    notify_task: Option<tokio::task::JoinHandle<()>>,
    scan_until: Option<tokio::time::Instant>,
    reported_adapter_on: bool,
    adapter_state_reported: bool,
    events: broadcast::Sender<TransportEvent>,
}

impl Actor {
    fn emit(&self, event: TransportEvent) {
        let _ = self.events.send(event);
    }

    /// Emits `AdapterState(on)` on the first report and whenever the state
    /// changes, so an off→on recovery re-reports on exactly once and a
    /// persistent off state is not re-emitted on every command. The first
    /// report must fire even when `on` is already `false` (the session starts
    /// optimistically on), otherwise a "no adapter" start would never disable
    /// scanning.
    fn report_adapter_state(&mut self, on: bool) {
        if self.adapter_state_reported && on == self.reported_adapter_on {
            return;
        }
        self.adapter_state_reported = true;
        self.reported_adapter_on = on;
        if !on {
            log::warn!("Bluetooth is off");
        }
        self.emit(TransportEvent::AdapterState(on));
    }

    /// Returns a usable adapter, opening one on first use. Reports
    /// `AdapterState(false)` and yields `None` when none is available or the
    /// adapter is powered off, so the caller can keep serving commands.
    async fn require_adapter(&mut self) -> Option<Adapter> {
        if let Some(adapter) = self.adapter.clone() {
            if adapter_is_on(&adapter).await {
                self.report_adapter_state(true);
                return Some(adapter);
            }
            self.report_adapter_state(false);
            return None;
        }

        let adapter = match open_adapter().await {
            Some(adapter) => adapter,
            None => {
                self.report_adapter_state(false);
                return None;
            }
        };
        self.adapter = Some(adapter.clone());
        if adapter_is_on(&adapter).await {
            self.report_adapter_state(true);
            Some(adapter)
        } else {
            self.report_adapter_state(false);
            None
        }
    }

    async fn start_scan(&mut self) {
        let Some(adapter) = self.require_adapter().await else {
            return;
        };
        match adapter.start_scan(ScanFilter::default()).await {
            Ok(()) => {
                self.peripherals.clear();
                self.scan_until = Some(tokio::time::Instant::now() + SCAN_WINDOW);
                self.emit(TransportEvent::ScanState(true));
            }
            Err(error) => {
                log::warn!("Bluetooth scan error: {error}");
                self.emit(TransportEvent::Error(format!("Scan Error: {error}")));
                self.emit(TransportEvent::ScanState(false));
            }
        }
    }

    async fn stop_scan(&mut self) {
        self.scan_until = None;
        if let Some(adapter) = self.adapter.clone()
            && let Err(error) = adapter.stop_scan().await
        {
            log::warn!("Bluetooth stop scan error: {error}");
        }
        self.emit(TransportEvent::ScanState(false));
    }

    /// Refreshes the cached peripherals and publishes them as scan results.
    async fn poll_scan(&mut self) {
        let Some(adapter) = self.adapter.clone() else {
            return;
        };
        let peripherals = match adapter.peripherals().await {
            Ok(peripherals) => peripherals,
            Err(error) => {
                log::warn!("Bluetooth peripherals error: {error}");
                return;
            }
        };
        let mut devices = Vec::with_capacity(peripherals.len());
        for peripheral in &peripherals {
            let id = peripheral.id().to_string();
            let properties = peripheral.properties().await.ok().flatten();
            devices.push(match properties {
                Some(properties) => ScanDevice {
                    id: DeviceId(id.clone()),
                    name: device_name(properties.local_name.as_deref(), &id),
                    rssi: properties.rssi,
                },
                None => ScanDevice {
                    id: DeviceId(id.clone()),
                    name: id,
                    rssi: None,
                },
            });
        }
        self.peripherals = peripherals;
        self.emit(TransportEvent::Devices(devices));
    }

    async fn find_peripheral(&mut self, id: &DeviceId) -> Option<Peripheral> {
        if let Some(peripheral) = self.peripherals.iter().find(|p| p.id().to_string() == id.0) {
            return Some(peripheral.clone());
        }
        if let Some(adapter) = self.adapter.clone()
            && let Ok(peripherals) = adapter.peripherals().await
            && let Some(peripheral) = peripherals.iter().find(|p| p.id().to_string() == id.0)
        {
            return Some(peripheral.clone());
        }
        None
    }

    async fn connect(&mut self, id: DeviceId) {
        self.scan_until = None;
        if let Some(adapter) = self.adapter.clone() {
            let _ = adapter.stop_scan().await;
        }

        let Some(peripheral) = self.find_peripheral(&id).await else {
            self.emit(TransportEvent::Error(format!(
                "Connection error: device {} not found",
                id.0
            )));
            return;
        };

        let connected = match tokio::time::timeout(CONNECT_TIMEOUT, peripheral.connect()).await {
            Ok(result) => result.map_err(|error| error.to_string()),
            Err(_) => Err("timed out".to_string()),
        };
        if let Err(error) = connected {
            self.emit(TransportEvent::Error(format!("Connection error: {error}")));
            return;
        }

        if let Err(error) = peripheral.discover_services().await {
            self.emit(TransportEvent::Error(format!("Connection error: {error}")));
            return;
        }

        for characteristic in peripheral.characteristics() {
            let properties = characteristic.properties;
            if properties.intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
                && let Err(error) = peripheral.subscribe(&characteristic).await
            {
                log::warn!("Bluetooth subscribe error: {error}");
            }
            if properties.contains(CharPropFlags::READ) {
                match peripheral.read(&characteristic).await {
                    Ok(data) => self.emit(TransportEvent::ReadValue {
                        uuid: characteristic.uuid.to_string(),
                        data,
                    }),
                    Err(error) => log::warn!("Bluetooth read error: {error}"),
                }
            }
        }

        let name = peripheral
            .properties()
            .await
            .ok()
            .flatten()
            .and_then(|properties| properties.local_name)
            .unwrap_or_else(|| id.0.clone());

        self.connected = Some(peripheral.clone());
        self.emit(TransportEvent::Connected { id, name });
        self.spawn_notifications(peripheral);
    }

    fn spawn_notifications(&mut self, peripheral: Peripheral) {
        if let Some(handle) = self.notify_task.take() {
            handle.abort();
        }
        let events = self.events.clone();
        self.notify_task = Some(tokio::task::spawn(async move {
            let mut stream = match peripheral.notifications().await {
                Ok(stream) => stream,
                Err(error) => {
                    log::warn!("Bluetooth notification error: {error}");
                    return;
                }
            };
            while let Some(data) = stream.next().await {
                let _ = events.send(TransportEvent::Notification {
                    uuid: data.uuid.to_string(),
                    data: data.value,
                });
            }
        }));
    }

    async fn disconnect(&mut self) {
        if let Some(handle) = self.notify_task.take() {
            handle.abort();
        }
        let was_connected = self.connected.is_some();
        if let Some(peripheral) = self.connected.take()
            && let Err(error) = peripheral.disconnect().await
        {
            log::warn!("Bluetooth disconnect error: {error}");
        }
        if was_connected {
            self.emit(TransportEvent::Disconnected);
        }
    }

    /// Finds a characteristic by UUID, preferring an exact service match.
    fn characteristic(
        peripheral: &Peripheral,
        service: &str,
        characteristic: &str,
    ) -> Option<btleplug::api::Characteristic> {
        let service = Uuid::parse_str(service).ok();
        let characteristic = Uuid::parse_str(characteristic).ok()?;
        let characteristics = peripheral.characteristics();
        let exact = service.and_then(|service| {
            characteristics
                .iter()
                .find(|c| c.uuid == characteristic && c.service_uuid == service)
                .cloned()
        });
        exact.or_else(|| {
            characteristics
                .into_iter()
                .find(|c| c.uuid == characteristic)
        })
    }

    async fn write(&mut self, service: String, characteristic: String, bytes: Vec<u8>) {
        let Some(peripheral) = self.connected.clone() else {
            self.emit(TransportEvent::Error("Not connected".into()));
            return;
        };
        let Some(target) = Self::characteristic(&peripheral, &service, &characteristic) else {
            self.emit(TransportEvent::Error(
                "Command Failed: characteristic not found".into(),
            ));
            return;
        };
        let kind = write_type(
            target
                .properties
                .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE),
        );
        if let Err(error) = peripheral.write(&target, &bytes, kind).await {
            self.emit(TransportEvent::Error(format!("Command Failed: {error}")));
        }
    }

    async fn read(&mut self, _service: String, characteristic: String) {
        let Some(peripheral) = self.connected.clone() else {
            self.emit(TransportEvent::Error("Not connected".into()));
            return;
        };
        let Some(target) = Self::characteristic(&peripheral, &_service, &characteristic) else {
            self.emit(TransportEvent::Error(
                "Command Failed: characteristic not found".into(),
            ));
            return;
        };
        match peripheral.read(&target).await {
            Ok(data) => self.emit(TransportEvent::ReadValue {
                uuid: target.uuid.to_string(),
                data,
            }),
            Err(error) => self.emit(TransportEvent::Error(format!("Command Failed: {error}"))),
        }
    }

    async fn handle(&mut self, command: TransportCommand) {
        match command {
            TransportCommand::StartScan { .. } | TransportCommand::Reconnect { .. } => {
                self.start_scan().await;
            }
            TransportCommand::StopScan => self.stop_scan().await,
            TransportCommand::Connect { id } => self.connect(id).await,
            TransportCommand::Disconnect => self.disconnect().await,
            TransportCommand::Write {
                service,
                characteristic,
                bytes,
            } => self.write(service, characteristic, bytes).await,
            TransportCommand::Read {
                service,
                characteristic,
            } => self.read(service, characteristic).await,
            TransportCommand::RequestPermissions => {}
        }
    }
}

async fn adapter_is_on(adapter: &Adapter) -> bool {
    matches!(adapter.adapter_state().await, Ok(CentralState::PoweredOn))
}

async fn open_adapter() -> Option<Adapter> {
    let manager = match Manager::new().await {
        Ok(manager) => manager,
        Err(error) => {
            log::warn!("Bluetooth manager error: {error}");
            return None;
        }
    };
    match manager.adapters().await {
        Ok(adapters) => adapters.into_iter().next(),
        Err(error) => {
            log::warn!("Bluetooth adapters error: {error}");
            None
        }
    }
}

async fn actor(
    mut commands: mpsc::UnboundedReceiver<TransportCommand>,
    events: broadcast::Sender<TransportEvent>,
    _wifi: watch::Sender<bool>,
) {
    let mut actor = Actor {
        adapter: None,
        peripherals: Vec::new(),
        connected: None,
        notify_task: None,
        scan_until: None,
        reported_adapter_on: false,
        adapter_state_reported: false,
        events,
    };
    actor.require_adapter().await;

    let mut ticker = tokio::time::interval(SCAN_TICK);
    loop {
        let deadline = actor.scan_until;
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => actor.handle(command).await,
                None => break,
            },
            _ = ticker.tick() => {
                if actor.scan_until.is_some() {
                    actor.poll_scan().await;
                } else if let Some(peripheral) = actor.connected.clone()
                    && !peripheral.is_connected().await.unwrap_or(true)
                {
                    actor.connected = None;
                    if let Some(handle) = actor.notify_task.take() {
                        handle.abort();
                    }
                    actor.emit(TransportEvent::Disconnected);
                }
            }
            _ = async {
                match deadline {
                    Some(until) => tokio::time::sleep_until(until).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                actor.stop_scan().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_and_write_flags_select_the_right_write_type() {
        assert_eq!(write_type(true), btleplug::api::WriteType::WithoutResponse);
        assert_eq!(write_type(false), btleplug::api::WriteType::WithResponse);
    }

    #[test]
    fn a_missing_local_name_falls_back_to_the_device_id() {
        assert_eq!(device_name(Some("BScooter"), "AA:BB"), "BScooter");
        assert_eq!(device_name(None, "AA:BB"), "AA:BB");
        assert_eq!(device_name(Some(""), "AA:BB"), "AA:BB");
    }
}
