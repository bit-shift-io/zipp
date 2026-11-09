//! Android transport: the Rust half of the `ZippBridge` JNI contract.
//!
//! Kotlin owns BLE, GPS and Wi-Fi and calls the `Java_...` entry points in
//! `crate::jni`; this module forwards `TransportCommand`s back to the static
//! `ZippBridge` methods over JNI and fans the callbacks out as core events.
//!
//! The two pure helpers below stay unconditional so the desktop `cargo test`
//! runner can cover them; everything that touches the JVM is gated to Android.

use zipp_core::gps::GpsStatus;
use zipp_core::i18n::Lang;

/// Maps a locale tag (`zh_CN`, `en_US`, ...) to a core language. Only wired
/// into `install` on Android, but shared with the desktop tests.
#[cfg_attr(not(any(target_os = "android", test)), allow(dead_code))]
pub fn lang_from_tag(tag: &str) -> Lang {
    Lang::from_tag(tag)
}

/// Maps the Kotlin GPS status strings to core statuses. Only wired into
/// `emit_gps_status` on Android, but shared with the desktop tests.
#[cfg_attr(not(any(target_os = "android", test)), allow(dead_code))]
pub fn gps_status(value: &str) -> GpsStatus {
    match value {
        "denied" => GpsStatus::Denied,
        "searching" => GpsStatus::Searching,
        "active" => GpsStatus::Active,
        _ => GpsStatus::Disabled,
    }
}

#[cfg(target_os = "android")]
mod imp {
    use std::sync::{Arc, Mutex, OnceLock};

    use futures::SinkExt;
    use iced::Subscription;
    use jni::objects::{GlobalRef, JValue};
    use jni::{JNIEnv, JavaVM};
    use tokio::sync::{broadcast, watch};
    use zipp_core::gps::{GpsStatus, GpsUpdate};
    use zipp_core::i18n::Lang;
    use zipp_core::transport::{Transport, TransportCommand, TransportEvent};

    use crate::message::Message;

    use super::{gps_status, lang_from_tag};

    /// The class looked up in `JNI_OnLoad`, fixed by the Kotlin package.
    pub const BRIDGE_CLASS: &str = "io/bitshift/zipp/ZippBridge";

    const CALLBACK_CAPACITY: usize = 256;
    const GPS_CAPACITY: usize = 64;
    const BACK_CAPACITY: usize = 8;

    /// The `ZippBridge` transport: every command reattaches the current thread
    /// to the cached VM and calls the matching static Kotlin method.
    pub struct AndroidTransport {
        vm: JavaVM,
        bridge: GlobalRef,
        events: broadcast::Sender<TransportEvent>,
        wifi: watch::Sender<bool>,
    }

    static VM: OnceLock<JavaVM> = OnceLock::new();
    static BRIDGE: OnceLock<GlobalRef> = OnceLock::new();
    static TRANSPORT: OnceLock<Arc<AndroidTransport>> = OnceLock::new();
    static LANG: OnceLock<Lang> = OnceLock::new();
    static BACK: OnceLock<broadcast::Sender<()>> = OnceLock::new();
    static GPS: OnceLock<broadcast::Sender<GpsUpdate>> = OnceLock::new();
    /// The last GPS status reported by Kotlin, so a speed-only callback can
    /// carry it in the `GpsUpdate`.
    static GPS_STATUS: OnceLock<Mutex<GpsStatus>> = OnceLock::new();

    impl AndroidTransport {
        /// Attaches the current thread and calls a static `ZippBridge` method,
        /// logging and clearing any exception instead of panicking.
        fn call(&self, method: &str, signature: &str, args: &[JValue]) {
            let Ok(mut env) = self.vm.attach_current_thread() else {
                return;
            };
            if let Err(error) = env.call_static_method(&self.bridge, method, signature, args) {
                let _ = env.exception_clear();
                log::warn!("ZippBridge.{method}: {error}");
            }
        }

        /// Calls a `void` method with no arguments.
        fn call_void(&self, method: &str) {
            self.call(method, "()V", &[]);
        }

        /// Calls a method taking a single `String`.
        fn call_string(&self, method: &str, value: &str) {
            let Ok(env) = self.vm.attach_current_thread() else {
                return;
            };
            match env.new_string(value) {
                Ok(string) => {
                    self.call(method, "(Ljava/lang/String;)V", &[(&string).into()]);
                }
                Err(error) => {
                    let _ = env.exception_clear();
                    log::warn!("ZippBridge.{method}: {error}");
                }
            }
        }

        /// Calls a method taking two `String`s.
        fn call_two_strings(&self, method: &str, first: &str, second: &str) {
            let Ok(env) = self.vm.attach_current_thread() else {
                return;
            };
            match (env.new_string(first), env.new_string(second)) {
                (Ok(first), Ok(second)) => {
                    self.call(
                        method,
                        "(Ljava/lang/String;Ljava/lang/String;)V",
                        &[(&first).into(), (&second).into()],
                    );
                }
                (_, _) => {
                    let _ = env.exception_clear();
                    log::warn!("ZippBridge.{method}: could not build arguments");
                }
            }
        }

        /// Calls `write(String, String, ByteArray)`.
        fn call_write(&self, service: &str, characteristic: &str, bytes: &[u8]) {
            let Ok(env) = self.vm.attach_current_thread() else {
                return;
            };
            let built = (
                env.new_string(service),
                env.new_string(characteristic),
                env.byte_array_from_slice(bytes),
            );
            match built {
                (Ok(service), Ok(characteristic), Ok(array)) => {
                    self.call(
                        "write",
                        "(Ljava/lang/String;Ljava/lang/String;[B)V",
                        &[(&service).into(), (&characteristic).into(), (&array).into()],
                    );
                }
                (_, _, _) => {
                    let _ = env.exception_clear();
                    log::warn!("ZippBridge.write: could not build arguments");
                }
            }
        }
    }

    impl Transport for AndroidTransport {
        fn send(&self, command: TransportCommand) {
            match command {
                TransportCommand::Reconnect { target } => self.call_string("reconnect", &target),
                TransportCommand::StartScan { target } => self.call_string("startScan", &target),
                TransportCommand::StopScan => self.call_void("stopScan"),
                TransportCommand::Connect { id } => self.call_string("connect", &id.0),
                TransportCommand::Disconnect => self.call_void("disconnect"),
                TransportCommand::Write {
                    service,
                    characteristic,
                    bytes,
                } => self.call_write(&service, &characteristic, &bytes),
                TransportCommand::Read {
                    service,
                    characteristic,
                } => self.call_two_strings("read", &service, &characteristic),
                TransportCommand::RequestPermissions => self.call_void("requestPermissions"),
            }
        }

        fn subscribe(&self) -> broadcast::Receiver<TransportEvent> {
            self.events.subscribe()
        }

        fn wifi(&self) -> watch::Receiver<bool> {
            self.wifi.subscribe()
        }
    }

    /// Caches the VM and the `ZippBridge` class. Called once from `JNI_OnLoad`
    /// on the thread that loaded the library, whose class loader is the app's.
    pub fn on_load(vm: JavaVM, env: &mut JNIEnv) {
        let class = match env.find_class(BRIDGE_CLASS) {
            Ok(class) => class,
            Err(error) => {
                let _ = env.exception_clear();
                log::warn!("ZippBridge class not found in JNI_OnLoad: {error}");
                return;
            }
        };
        let bridge = match env.new_global_ref(class) {
            Ok(bridge) => bridge,
            Err(error) => {
                let _ = env.exception_clear();
                log::warn!("could not cache global ZippBridge in JNI_OnLoad: {error}");
                return;
            }
        };
        let _ = VM.set(vm);
        let _ = BRIDGE.set(bridge);
    }

    /// Builds the broadcast/watch channels and stores the transport. Returns an
    /// error when `JNI_OnLoad` did not cache the bridge class; the caller logs
    /// it and never panics.
    pub fn install(env: &mut JNIEnv, locale: &str) -> Result<(), String> {
        let vm = env.get_java_vm().map_err(|error| error.to_string())?;
        let _ = LANG.set(lang_from_tag(locale));
        let (events, _) = broadcast::channel(CALLBACK_CAPACITY);
        let (wifi, _) = watch::channel(false);
        let (back, _) = broadcast::channel(BACK_CAPACITY);
        let _ = BACK.set(back);
        let _ = gps_sender();
        let _ = GPS_STATUS.set(Mutex::new(GpsStatus::Disabled));
        let bridge = BRIDGE.get().ok_or("ZippBridge class not cached")?.clone();
        let transport = Arc::new(AndroidTransport {
            vm,
            bridge,
            events,
            wifi,
        });
        let _ = TRANSPORT.set(transport);
        Ok(())
    }

    pub fn transport() -> Option<Arc<AndroidTransport>> {
        TRANSPORT.get().cloned()
    }

    pub fn current_lang() -> Lang {
        LANG.get().copied().unwrap_or_default()
    }

    pub fn back_sender() -> Option<broadcast::Sender<()>> {
        BACK.get().cloned()
    }

    pub fn gps_sender() -> broadcast::Sender<GpsUpdate> {
        GPS.get_or_init(|| broadcast::channel(GPS_CAPACITY).0)
            .clone()
    }

    /// Emits a transport event into the broadcast channel, if installed.
    pub fn emit(event: TransportEvent) {
        if let Some(transport) = TRANSPORT.get() {
            let _ = transport.events.send(event);
        }
    }

    /// Stores and emits the Wi-Fi state on both the watch and broadcast
    /// channels so the Entry policy and the UI both see it.
    pub fn emit_wifi(value: bool) {
        if let Some(transport) = TRANSPORT.get() {
            // `send` fails without a live receiver and would leave the watch at
            // its initial value; `send_replace` always stores the new state.
            transport.wifi.send_replace(value);
            let _ = transport.events.send(TransportEvent::Wifi(value));
        }
    }

    /// Stores the status and forwards a status-only GPS update.
    pub fn emit_gps_status(value: &str) {
        let status = gps_status(value);
        if let Some(slot) = GPS_STATUS.get() {
            match slot.lock() {
                Ok(mut last) => *last = status,
                Err(poisoned) => *poisoned.into_inner() = status,
            }
        }
        if let Some(sender) = GPS.get() {
            let _ = sender.send(GpsUpdate {
                status,
                speed_mps: None,
            });
        }
    }

    /// Forwards a speed update carrying the last reported status.
    pub fn emit_gps_speed(mps: f64) {
        let status = GPS_STATUS
            .get()
            .map(|slot| match slot.lock() {
                Ok(last) => *last,
                Err(poisoned) => *poisoned.into_inner(),
            })
            .unwrap_or(GpsStatus::Active);
        if let Some(sender) = GPS.get() {
            let _ = sender.send(GpsUpdate {
                status,
                speed_mps: Some(mps),
            });
        }
    }

    pub fn back_subscription() -> Subscription<Message> {
        Subscription::run(back_stream)
    }

    fn back_stream() -> impl futures::Stream<Item = Message> {
        iced::stream::channel(BACK_CAPACITY, async move |mut output| {
            let Some(sender) = back_sender() else {
                return;
            };
            let mut receiver = sender.subscribe();
            loop {
                match receiver.recv().await {
                    Ok(()) => {
                        if output.send(Message::Back).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }

    pub fn gps_subscription() -> Subscription<Message> {
        Subscription::run(gps_stream)
    }

    fn gps_stream() -> impl futures::Stream<Item = Message> {
        iced::stream::channel(GPS_CAPACITY, async move |mut output| {
            let mut receiver = gps_sender().subscribe();
            loop {
                match receiver.recv().await {
                    Ok(update) => {
                        if output.send(Message::Gps(update)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }
}

#[cfg(target_os = "android")]
pub use imp::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags_map_to_core_languages() {
        assert_eq!(lang_from_tag("zh_CN"), zipp_core::i18n::Lang::Zh);
        assert_eq!(lang_from_tag("en_US"), zipp_core::i18n::Lang::En);
    }

    #[test]
    fn gps_status_strings_match_the_kotlin_values() {
        assert_eq!(gps_status("disabled"), zipp_core::gps::GpsStatus::Disabled);
        assert_eq!(gps_status("denied"), zipp_core::gps::GpsStatus::Denied);
        assert_eq!(
            gps_status("searching"),
            zipp_core::gps::GpsStatus::Searching
        );
        assert_eq!(gps_status("active"), zipp_core::gps::GpsStatus::Active);
        assert_eq!(gps_status("junk"), zipp_core::gps::GpsStatus::Disabled);
    }
}
