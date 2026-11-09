//! Android JNI entry points for the Kotlin `ZippBridge`.
//!
//! Kotlin owns the BLE/GPS/Wi-Fi lifecycle and calls these functions; this
//! module only converts the arguments into core events and forwards them to
//! [`crate::platform::android`]. Every conversion uses `unwrap_or_default` and
//! clears a pending exception, so a Java failure never aborts the process.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;

use jni::objects::{JByteArray, JObject, JString};
use jni::sys::{JNI_ERR, JNI_VERSION_1_6, jboolean, jdouble, jint};
use jni::{JNIEnv, JavaVM};
use zipp_core::protocol::target_device_name;
use zipp_core::transport::{DeviceId, ScanDevice, TransportEvent};

use crate::platform::android;

/// Scan results for the current scan window, keyed by address so duplicate
/// advertisements collapse and each snapshot is stably ordered.
static DEVICES: Mutex<BTreeMap<String, ScanDevice>> = Mutex::new(BTreeMap::new());

fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("zipp")
            .with_max_level(log::LevelFilter::Info),
    );
}

/// Reads a Java string, clearing a pending exception on failure.
fn read_string(env: &mut JNIEnv, value: &JString) -> String {
    match env.get_string(value) {
        Ok(text) => text.into(),
        Err(_) => {
            let _ = env.exception_clear();
            String::new()
        }
    }
}

/// Reads a Java byte array, clearing a pending exception on failure.
fn read_bytes(env: &mut JNIEnv, value: &JByteArray) -> Vec<u8> {
    match env.convert_byte_array(value) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = env.exception_clear();
            Vec::new()
        }
    }
}

/// Locks the scan map, recovering from a poisoned lock instead of panicking.
fn lock_devices() -> std::sync::MutexGuard<'static, BTreeMap<String, ScanDevice>> {
    match DEVICES.lock() {
        Ok(devices) => devices,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Export for `System.loadLibrary("zipp")`. Runs on the thread that loaded the
/// library, whose class loader is the app's, so `FindClass` works here and only
/// here; later threads use the cached global reference.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jint {
    let vm = match unsafe { JavaVM::from_raw(vm) } {
        Ok(vm) => vm,
        Err(_) => return JNI_ERR,
    };
    init_logging();
    // `JavaVM::get_env` borrows the VM it is called on while `android::on_load`
    // takes ownership of `vm`, so read the environment through a second handle
    // to the same JVM pointer.
    if let Ok(holder) = unsafe { JavaVM::from_raw(vm.get_java_vm_pointer()) }
        && let Ok(mut env) = holder.get_env()
    {
        android::on_load(vm, &mut env);
    }
    JNI_VERSION_1_6
}

/// Called by `MainActivity.onCreate` before `super.onCreate`: install the
/// locale and transport, then let Kotlin request permissions, Wi-Fi and GPS.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_nativeInit(
    mut env: JNIEnv,
    _this: JObject,
    locale: JString,
) {
    init_logging();
    let locale = read_string(&mut env, &locale);
    if let Err(error) = android::install(&mut env, &locale) {
        let _ = env.exception_clear();
        log::warn!("ZippBridge install failed: {error}");
    }
}

/// The Back button/gesture, from `MainActivity.onBackPressed`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_nativeBack(_env: JNIEnv, _this: JObject) {
    if let Some(sender) = android::back_sender() {
        let _ = sender.send(());
    }
}

/// One advertisement, already filtered to the target name by Kotlin.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onScanResult(
    mut env: JNIEnv,
    _this: JObject,
    address: JString,
    name: JString,
    rssi: jint,
) {
    let address = read_string(&mut env, &address);
    let name = read_string(&mut env, &name);
    if address.is_empty() {
        return;
    }
    let snapshot = {
        let mut devices = lock_devices();
        devices.insert(
            address.clone(),
            ScanDevice {
                id: DeviceId(address),
                name,
                rssi: Some(rssi as i16),
            },
        );
        devices.values().cloned().collect::<Vec<_>>()
    };
    android::emit(TransportEvent::Devices(snapshot));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onScanState(
    _env: JNIEnv,
    _this: JObject,
    scanning: jboolean,
) {
    let scanning = scanning != 0;
    if scanning {
        lock_devices().clear();
    }
    android::emit(TransportEvent::ScanState(scanning));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onAdapterState(
    _env: JNIEnv,
    _this: JObject,
    on: jboolean,
) {
    android::emit(TransportEvent::AdapterState(on != 0));
}

/// The bridge only forwards the address; the name is the target because the
/// scan filter accepts exactly that name.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onConnectionState(
    mut env: JNIEnv,
    _this: JObject,
    address: JString,
    connected: jboolean,
) {
    let address = read_string(&mut env, &address);
    let event = if connected != 0 {
        TransportEvent::Connected {
            id: DeviceId(address),
            name: target_device_name(),
        }
    } else {
        TransportEvent::Disconnected
    };
    android::emit(event);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onNotification(
    mut env: JNIEnv,
    _this: JObject,
    uuid: JString,
    bytes: JByteArray,
) {
    let uuid = read_string(&mut env, &uuid);
    let data = read_bytes(&mut env, &bytes);
    android::emit(TransportEvent::Notification { uuid, data });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onRead(
    mut env: JNIEnv,
    _this: JObject,
    uuid: JString,
    bytes: JByteArray,
) {
    let uuid = read_string(&mut env, &uuid);
    let data = read_bytes(&mut env, &bytes);
    android::emit(TransportEvent::ReadValue { uuid, data });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onWifiState(
    _env: JNIEnv,
    _this: JObject,
    connected: jboolean,
) {
    android::emit_wifi(connected != 0);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onGpsStatus(
    mut env: JNIEnv,
    _this: JObject,
    status: JString,
) {
    let status = read_string(&mut env, &status);
    android::emit_gps_status(&status);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onGpsSpeed(
    _env: JNIEnv,
    _this: JObject,
    mps: jdouble,
) {
    android::emit_gps_speed(mps);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_bitshift_zipp_ZippBridge_onError(
    mut env: JNIEnv,
    _this: JObject,
    message: JString,
) {
    let message = read_string(&mut env, &message);
    android::emit(TransportEvent::Error(message));
}
