# Zipp — Flutter to iced port — Design

**Date:** 2026-11-08
**Status:** Draft for approval
**Repository:** `/home/bronson/Projects/zipp`
**App root:** `/home/bronson/Projects/zipp/app`
**Reference shells:** `pige` (2026-11-04) and `qlaunch` (2026-11-01)

---

## 1. Intent

Zipp's app is a small Flutter UI over a Rust protocol core. It has two screens
(a scooter dashboard and a log terminal), a Bluetooth LE connection to a
`BScooter`, GPS speed, and an HTTP "Entry" button. The Flutter layer is being
replaced with Rust + iced so the whole app is one language and one toolchain,
matching the pattern already proven in `pige` and `qlaunch`:

- a platform-independent `zipp_core` crate owns protocol and state,
- a `zipp` crate owns the iced UI, the desktop binary and the Android JNI
  exports,
- a Kotlin `ZippBridge` owns the Android OS APIs (BLE, GPS, permissions,
  connectivity) and only answers questions or performs actions,
- an `android/` Gradle shell hosts `IcedActivity`,
- Flutter, Dart, `flutter_rust_bridge` and the Flutter Gradle plugin are
  removed from the active tree.

The reason is control and consistency: no Flutter runtime or bridge, the core
logic already exists in Rust, and the Android build becomes the same
Gradle/NDK/`iced_android` stack as the other projects.

### Success looks like

- `cargo test --workspace` passes from `app/rust`.
- `./run.sh` opens the Zipp dashboard on Linux. The Entry button sends the
  request; the scanner reports a missing/off adapter instead of crashing; GPS
  shows disabled.
- `bash scripts/build.sh release` produces
  `build/app/outputs/apk/release/app-release.apk`. Installed on the test phone,
  it requests BLE/location access, scans for `BScooter`, auto-connects, shows
  live speed/battery/clock/temperature/cruise/odometer, sends every button,
  streams UART RX into the log, and shows GPS speed.
- No `pubspec.yaml`, Dart source, `flutter_rust_bridge`, Flutter Gradle plugin,
  `ios/`, `linux/`, `macos/`, `windows/`, `web/` or `rust_builder/` remains under
  `app/`.
- A Flutter/Dart grep over active sources and docs (excluding `old/` and git
  history) finds nothing.

---

## 2. Non-goals

- iOS, web, Windows or macOS. Their Flutter shells are deleted, not ported.
- A visual redesign. Layout, colours, font sizes, labels and interaction are
  preserved from the Flutter screens, with two deliberate fixes called out in
  §5.
- Persisted settings or state. The Flutter app had none; the iced app keeps the
  same in-memory session.
- Background BLE/GPS work while the app is not visible.
- A release signing key. The build falls back to the local debug key unless
  `android/keystore.properties` exists, following `pige`.

---

## 3. Current state

### Files to replace

| Path | What it is | Fate |
| :--- | :--- | :--- |
| `app/lib/` | 11 Dart UI/service files + i18n JSON + generated strings | delete |
| `app/rust/` | `rust_lib_zipp` with frb; `src/api/protocol.rs` holds the logic | move logic, delete frb |
| `app/rust_builder/` | cargokit bridge | delete |
| `app/android/` | Flutter Gradle plugin, `FlutterActivity`, launch themes | replace |
| `app/{ios,linux,macos,windows,web}/` | Flutter platform shells | delete |
| `app/pubspec.*`, `analysis_options.yaml`, `.metadata`, `flutter_rust_bridge.yaml`, `build.sh`, `run.sh`, `.gitignore` | Flutter tooling | replace |
| `app/assets/app_icon.{png,svg}` | app icon source | keep |
| `app/android/.../res/mipmap-*` | launcher PNGs | keep |
| `old/` | earlier Rust/Slint and Java attempts | keep (history, not Flutter) |

### Behaviour to preserve

- Dashboard fixed 480 dp tall, five rows: four buttons; temperature / clock /
  battery; indicators / speed / cruise bars; odometer / power / clock; four
  buttons.
- Bottom connectivity status and scanner list.
- Log page: title and back, terminal, clear button, UART input.
- BLE target name `BScooter`; auto-connect from scan results; reconnect every
  5 s; 15 s scans; bonded-device check first; 10 s connect timeout; subscribe to
  every notify/indicate characteristic; read every readable characteristic;
  parse by UUID; write commands with `writeWithoutResponse` when available.
- GPS speed `m/s * 3.6`; status `disabled / denied / searching / active`.
- Entry `http://192.168.1.3:1880/entry`; desktop sends immediately; mobile waits
  for Wi-Fi; in-flight requests are deduplicated.
- Logs `HH:MM message`, maximum 200, auto-scroll, clear.
- English and Chinese, device locale.

---

## 4. Architecture

### 4.1 Workspace

`app/rust/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["zipp_core", "zipp"]

[workspace.package]
version = "2026.11.8"

[patch.crates-io]
android-activity = { git = "https://github.com/Santuzius/android-activity", tag = "iced_android-v0.1.1" }

[profile.release]
strip = "debuginfo"
```

Pinned stack, identical to `pige`/`qlaunch`:

- `iced` from `https://github.com/Santuzius/iced`, tag `iced_android-v0.1.1`,
  features `wgpu`, `tokio`, `image`, `wayland`, `advanced`, `web-colors`,
  `canvas`.
- `iced_android` from `https://github.com/Santuzius/iced_android`, tag
  `v0.1.1`.
- Rust edition 2024, Android NDK `28.2.13676358`, JDK `/opt/jbr-21`,
  `minSdk 34`, `targetSdk 36`, arm64 only, 16 KiB page flag
  `-C link-arg=-Wl,-z,max-page-size=16384`.

### 4.2 Crates

**`zipp_core`** (platform-independent, `rlib`):

| Module | Responsibility |
| :--- | :--- |
| `protocol.rs` | `ScooterState`, `ScooterCommand`, `BleAction`, `ParseResult`, `parse_characteristic_data`, `create_command_bytes`, `get_command_action`, `target_device_name` |
| `transport.rs` | `Transport` trait, `TransportCommand`, `TransportEvent`, `DeviceId`, `ScanDevice` |
| `session.rs` | `ScooterSession`: all dashboard/scan/log/GPS/entry state and pure transitions |
| `i18n.rs` | `Lang`, `Strings`, `EN`, `ZH`, `from_tag`, `detect` |
| `gps.rs` | `GpsStatus`, `GpsUpdate`, `mps_to_kmh` |
| `entry.rs` | entry URL, pure "send now vs wait for Wi-Fi" decision |
| `log.rs` | `LogBuffer` (200-line cap, `HH:MM` formatting, clear) |
| `lib.rs` | re-exports and `SCHEMA`-free public surface |

**`zipp`** (iced UI, `cdylib` + `rlib` + desktop `bin`):

| Module | Responsibility |
| :--- | :--- |
| `lib.rs` | `run()`, `android_main`, module wiring |
| `main.rs` | desktop binary |
| `app.rs` | `App`: session + UI state; opens the single window |
| `message.rs` | `Message` |
| `update.rs` | Elm update, transport/HTTP/GPS tasks, subscriptions |
| `view.rs` | root page composition and insets |
| `theme.rs` | the only source of colours, spacing, radii and type |
| `fonts.rs` | platform default, bundled icon font, desktop CJK probe |
| `icons.rs` | Material icon codepoints |
| `components/` | `button.rs`, `stat.rs`, `scanner.rs`, `log.rs`, `chrome.rs`, `mod.rs` |
| `platform/` | `mod.rs`, `desktop.rs`, `android.rs` |
| `jni.rs` | Android JNI exports (`cfg(target_os = "android")`) |

`zipp` depends on `zipp_core`, iced, iced_android, tokio, futures, log; on
Android also `android_logger` and `jni`; on desktop `btleplug` and `reqwest`.
`zipp_core` depends only on `chrono`, `serde` (optional, for debug), `tokio`
(sync only), and `thiserror`.

### 4.3 Data flow

`App` owns a `ScooterSession` and an `Arc<dyn Transport>`.

```text
button/input
  -> Message
  -> App::update
  -> session mutation + Task
  -> Transport::send(TransportCommand)
  -> platform actor (btleplug or JNI)
  -> TransportEvent
  -> broadcast channel
  -> Subscription -> Message::Transport(event)
  -> session.apply(event, now)
  -> view
```

No mutable global state except the Android transport handle, installed by
`nativeInit` before `android_main` starts the iced window.

### 4.4 Transport contract

```rust
pub trait Transport: Send + Sync {
    fn send(&self, command: TransportCommand);
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TransportEvent>;
    fn wifi(&self) -> tokio::sync::watch::Receiver<bool>;
}

pub enum TransportCommand {
    Reconnect { target: String }, // Android: bonded first, then scan
    StartScan { target: String },
    StopScan,
    Connect { id: DeviceId },
    Disconnect,
    Write { service: String, characteristic: String, bytes: Vec<u8> },
    Read { service: String, characteristic: String },
    RequestPermissions,
}

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
```

`ScanDevice` is `{ id: DeviceId, name: String, rssi: Option<i16> }`. `DeviceId`
wraps the Android MAC address or the `btleplug` peripheral id as a `String`.

`ScooterSession` is pure and testable. It owns:

- `state: Option<ScooterState>` (stays `None` until the first parse, preserving
  Flutter's default display),
- `devices`, `connected`, `scanning`, `connecting`, `adapter_on`,
- `logs: LogBuffer`,
- `gps_status`, `gps_speed_kmh`,
- `entry_pending`.

`apply(event, now) -> Vec<TransportCommand>` handles auto-connect, clearing scan
results when a scan starts, logging, and state updates. `begin_scan`,
`connect_device`, `disconnect`, `send_command`, and `tick` return the commands
the UI must send. Everything else is a pure state change.

---

## 5. UI parity

### 5.1 Theme tokens

Dark only, black page background, exactly the Flutter values:

| Token | Value |
| :--- | :--- |
| `background` | `#000000` |
| `text` | `#FFFFFF` |
| `muted` | `#CCCCCC` |
| `disabled` | `#333333` |
| `active_fill` | `#333333` |
| `border` | white at 24 % alpha |
| `card` | white at 10 % alpha |
| `log_text` | `#CCCCCC` |
| `radius` | 12 |
| `button_border` | 2 |

Type scale: speed 100; top stats and clock 35 (am/pm 25); bottom stats and
indicators 26; units 24; button labels 20; list titles and UART input 18; log
lines and list subtitles 16; GPS label and device id 12. Components read
`theme`; no module invents a colour or size.

### 5.2 Dashboard (480 dp)

1. **Row 1 (90 dp):** Power, Alarm, Lights, Entry. Four equal columns, 4 dp
   padding. Non-action buttons fill `#333333` when on; all have a 2 px white-24
   border and 12 radius. Label 20 white centred; `ON`/`OFF` 20 white for
   toggles only.
2. **Row 2:** temperature left (`XX°C`), 12-hour clock centred with `am`/`pm`,
   battery right (`N%`). Padding 4 vertical / 10 horizontal, size 35.
3. **Divider:** white-24, 2 px.
4. **Row 3 (expanded):**
   - Left 100 dp: Brake, Cruise, Park indicators, 26; active white, inactive
     `#333333`.
   - Centre: GPS icon + `GPS` 12 only when active/searching; speed `00` at 100
     with `km/h` 24; throttle `0000 mv` 24.
   - Right 100 dp: five cruise bars, widths `20 + level*15`, full height,
     active `#CCCCCC`, inactive `#333333`, 100 radius, 4 dp vertical margin.
5. **Divider.**
6. **Row 4:** odometer `0000 km` left, battery power `0000 w` centre, clock
   `HH:MM` right; 26; padding 8 / 10.
7. **Row 5 (90 dp):** Sport, Cruise Down, Cruise Up, Horn.

Defaults when `state` is `None`: temperature `00`, battery `0`, speed `00`,
throttle `0000`, odometer `0000`, battery power `0000`, clock `00`, and
Brake/Cruise/Park shown active; Power/Alarm/Lights/Sport shown off. When GPS is
active, the speed becomes `round(kmh)` left-padded to two digits.

### 5.3 Connectivity and scanner

- Status row: icon 32, status text 20 bold, device id 12 white-54, scan/stop
  icon 28/32. Tapping the row opens the log page.
- Divider white-24.
- Connected card (`#FFFFFF10`): bluetooth-connected icon, device name 18,
  `Disconnect` button.
- Device rows: bluetooth icon, name 18, address 14, tap to connect.
- `No devices found` when the list is empty, not scanning and not connected.
- The list scrolls and shrinks to fit the remaining window height.

**Deliberate fix 1:** the Flutter scan button shows a stop icon while scanning
but still calls `scan()`. The port makes that button stop the scan, matching the
icon and intent. Everything else about scanning is unchanged.

### 5.4 Log page

- Header: back icon, title 28 bold, padding 16 horizontal / 12 vertical.
- Divider.
- Terminal: black, log lines 16 muted, line height 1.2, padding 10; clear
  button top-right (`#333333`, radius 4, white 16).
- Input row 60 dp, top border white-24; input 18 white, hint `...` white-24 20;
  chevron send 30 white in `#333333`.
- Auto-scroll to the newest line after a log arrives.
- Dashboard and log are two pages. The Flutter app slides over 300 ms. The port
  uses `iced::widget::float` to offset the outgoing/incoming page over 300 ms
  if it stays simple; otherwise it swaps instantly and this is recorded as a
  minor deviation. (Implementation plan decides after a spike; parity is the
  default.)

**Deliberate fix 2:** the Flutter `LogPage` calls `onBack` through an
`arrow_back_ios` icon but no Android back handling exists. The port maps the
system Back gesture/button to the same back action when the log page is open.

### 5.5 i18n

`Lang::En` and `Lang::Zh`, with the exact current strings:

| Key | English | Chinese |
| :--- | :--- | :--- |
| `dashboard.power` | Power | 电源 |
| `dashboard.alarm` | Alarm | 报警器 |
| `dashboard.lights` | Lights | 车灯 |
| `dashboard.sport` | Sport | 运动模式 |
| `dashboard.cruise` | Cruise | 定速巡航 |
| `dashboard.park` | Park | 驻车 |
| `dashboard.brake` | Brake | 刹车 |
| `dashboard.horn` | Horn | 喇叭 |
| `dashboard.entry` | Entry | 门禁 |
| `dashboard.cruise_up` | Cruise\nUp | 巡航\n加速 |
| `dashboard.cruise_down` | Cruise\nDown | 巡航\n减速 |
| `dashboard.units.kmh` | km/h | km/h |
| `dashboard.units.km` | km | km |
| `dashboard.units.celsius` | °C | °C |
| `dashboard.units.mv` | mv | mv |
| `dashboard.units.watts` | w | w |
| `dashboard.indicators.on` | ON | 开 |
| `dashboard.indicators.off` | OFF | 关 |
| `scanner.title` | Log | 日志 |
| `scanner.scanning` | Scanning... | 扫描中... |
| `scanner.connecting` | Connecting... | 连接中... |
| `scanner.disconnected` | Disconnected | 已断开 |
| `scanner.connect` | Connect | 连接 |
| `scanner.disconnect` | Disconnect | 断开 |
| `scanner.unknown` | Unknown Device | 未知设备 |
| `scanner.nearby` | Nearby Devices | 附近设备 |
| `scanner.log` | Log | 日志 |
| `scanner.clear` | clear | 清除 |

Detection: Android passes `Locale.getDefault().language` to `nativeInit`;
desktop checks `LC_ALL`, then `LC_MESSAGES`, then `LANG`; `zh*` selects Chinese,
everything else English. There is no runtime language switch, as before.

### 5.6 Icons and fonts

- Bundle `MaterialIcons-Regular.otf` and its license (copied from `pige`).
- Glyphs needed: gps_fixed, gps_not_fixed, bluetooth, bluetooth_connected,
  bluetooth_disabled, bluetooth_searching, search, stop, arrow_back_ios,
  chevron_right. Codepoints are verified with `fontTools` against the bundled
  font, as `pige` does.
- Android fonts come from `iced_android::fonts()`; it does **not** include CJK,
  so `zipp`'s own probe registers `/system/fonts/NotoSansCJK-Regular.ttc`.
- Desktop CJK: probe the common Noto CJK / WQY / AR PL paths and register the
  first readable font. If none exists, Chinese falls back to tofu; the
  technical notes record the result. This is the one platform-font risk.

---

## 6. Protocol core

The existing `app/rust/src/api/protocol.rs` is moved verbatim in behaviour into
`zipp_core/src/protocol.rs`, minus `serde` derives and `#[frb]` annotations.

- `ScooterState` fields and `Default` stay exactly as they are.
- `ScooterCommand` variants stay exactly as they are.
- `parse_characteristic_data` keeps the exact UUID normalisation:
  - a 36-character standard UUID (`0000xxxx-0000-1000-8000-00805f9b34fb`) maps
    to its four hex digits,
  - any other 36-character UUID is lowercased and kept whole (UART),
  - anything else is lowercased.
- The mapping stays: `1001` power, `1003` lights, `1004` alarm, `1005` sport,
  `2001` speed, `2002` throttle (LE u16), `2003` odometer (LE u16, `{:04}`),
  `2004` temperature, `2005` clock minutes, `2006` clock hours, `2007` brake,
  `2008` park brake, `2009` cruise level, `2a19` battery level, `2b05` battery
  power (LE u16), `0003`/`6e400003-...` UART RX.
- `create_command_bytes` keeps the exact byte values:
  Power off `0` / on `183`, Alarm off `0` / on `205`, Sport off `0` / on `1`,
  Lights off `0` / on `1`, CruiseUp `1`, CruiseDown `0`, Horn `1`, SetUart =
  UTF-8 bytes.
- `get_command_action` keeps the exact service/characteristic pairs and expands
  four-character UUIDs against `0000xxxx-0000-1000-8000-00805F9B34FB`; custom
  UART UUIDs stay lowercase.
- `target_device_name()` returns `BScooter`.

Unit tests cover every mapping, every command, short/empty data, unknown UUIDs,
uppercase and lowercase UUIDs, custom UART UUIDs, and the four-digit expansion.

---

## 7. Entry, GPS, logging

### Entry

`zipp_core::entry` owns the URL and the pure decision: on desktop send now;
on Android send now if `wifi` is true, otherwise wait for the next `Wifi(true)`.
`App::update` sets `entry_pending`, spawns the request as a `Task`, and clears
it when the request finishes. Desktop uses `reqwest` with default features off
(plain HTTP); Android uses the same crate. The response status is written to
the `log` crate (stdout/logcat), not to the in-app log, matching Flutter's
`debugPrint`.

### GPS

`zipp_core::gps` defines `GpsStatus` and `mps_to_kmh(mps) = mps * 3.6`.

- Android: `LocationManager` with `GPS_PROVIDER`, accuracy best-for-navigation,
  distance filter 0. Status is `Searching` after a successful start, `Active` on
  the first fix, `Denied`/`Disabled` on permission/provider failures.
- Desktop: `Disabled`, exactly as the Flutter service skipped Linux.

### Logging

`LogBuffer` is a `VecDeque<String>` capped at 200. Every line is
`HH:MM message` using local time (`chrono`). Adding a line schedules a scroll to
the bottom. `clear()` empties it. Bluetooth service messages keep the exact
Flutter text (`start scan`, `stop scan`, `connecting: <name>`,
`connected: <name>`, `disconnected from device`, `Not connected`,
`Connection error: ...`, `Command Failed: ...`, `Bluetooth is off`,
`Permissions denied for scanning`, `Scan Error: ...`).

---

## 8. Android shell

### 8.1 Activity

`android/app/src/main/kotlin/io/bitshift/zipp/MainActivity.kt`:

```kotlin
class MainActivity : IcedActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        System.loadLibrary("zipp")
        ZippBridge.install(this, applicationContext)
        ZippBridge.nativeInit(Locale.getDefault().language)
        super.onCreate(savedInstanceState)
        window.setFormat(PixelFormat.RGBA_8888)
        ZippBridge.requestPermissions()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int, permissions: Array<String>, grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        ZippBridge.onPermissionResult(grantResults.all { it == PackageManager.PERMISSION_GRANTED })
    }

    override fun onBackPressed() {
        ZippBridge.nativeBack()
    }

    override fun onDestroy() {
        ZippBridge.shutdown()
        super.onDestroy()
    }
}
```

`nativeInit` is an instance external on the `ZippBridge` object, so its Rust
symbol is `Java_io_bitshift_zipp_ZippBridge_nativeInit` with a `JObject` second
parameter. `nativeBack` dispatches the same `Message::Back` path as the iced
keyboard back.

### 8.2 Bridge

`ZippBridge.kt` is a Kotlin `object` holding the activity (weak) and application
context. It owns:

- `BluetoothManager`, `BluetoothAdapter`, `BluetoothLeScanner`, `ScanCallback`,
  `BluetoothGatt`, `BluetoothGattCallback`,
- a `UUID -> BluetoothGattCharacteristic` map built after service discovery,
- `LocationManager` and its listener,
- `ConnectivityManager` and a default-network callback.

Rust calls these `@JvmStatic` methods: `startScan(target)`, `stopScan()`,
`connect(address)`, `disconnect()`, `write(service, characteristic, bytes)`,
`read(service, characteristic)`, `requestPermissions()`, `startGps()`,
`stopGps()`, `registerWifi()`. Kotlin also exposes `shutdown()` for
`MainActivity.onDestroy()` to stop scanning, close the GATT and stop GPS.

Kotlin calls these `external fun` callbacks: `onScanResult(address, name, rssi)`,
`onScanState(scanning)`, `onAdapterState(on)`,
`onConnectionState(address, connected)`, `onNotification(uuid, bytes)`,
`onRead(uuid, bytes)`, `onWifiState(connected)`, `onGpsStatus(status)`,
`onGpsSpeed(mps)`, `onError(message)`.

BLE details:

- Scan uses `ScanFilter.Builder().setDeviceName(target)` and
  `SCAN_MODE_LOW_LATENCY`; a posted 15 s timeout calls `stopScan()`.
- Connect uses `device.connectGatt(context, false, callback, TRANSPORT_LE)`.
- On `onServicesDiscovered`, every NOTIFY/INDICATE characteristic gets
  `setCharacteristicNotification(true)` and a CCCD write
  (`00002902-0000-1000-8000-00805f9b34fb`); every READ characteristic gets
  `readCharacteristic`.
- Writes pick `WRITE_TYPE_NO_RESPONSE` when the characteristic has
  `PROPERTY_WRITE_NO_RESPONSE`, otherwise `WRITE_TYPE_DEFAULT`.
- Disconnect calls `disconnect()` then `close()`.

Permissions are requested from `MainActivity`:
`BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, `ACCESS_FINE_LOCATION`. A denial is
reported to Rust, which logs `Permissions denied for scanning` and does not
scan.

GPS uses `LocationManager.GPS_PROVIDER`; the listener reports status and
`speed` in m/s, which Rust converts to km/h.

Wi-Fi uses `ConnectivityManager.registerDefaultNetworkCallback` and
`NetworkCapabilities.hasTransport(TRANSPORT_WIFI)`; the current state is
reported immediately on registration and on every change.

### 8.3 JNI plumbing

`zipp/src/jni.rs`:

- `JNI_OnLoad` caches the `JavaVM` and a global reference to
  `io/bitshift/zipp/ZippBridge`.
- Every Rust -> Kotlin call attaches the current thread and uses
  `call_static_method`; every Kotlin -> Rust callback attaches its own thread.
- `nativeInit` installs the process-wide Android `Transport` (a broadcast
  sender for events and direct JNI calls for commands), stores the locale and
  sets up `android_logger` with tag `zipp`.
- `nativeBack` forwards the system Back action to the iced message stream.
- `android_main` calls `iced_android::init(app)` and then `run()`.

### 8.4 Manifest and Gradle

Manifest:

- `INTERNET`, `ACCESS_NETWORK_STATE`, `BLUETOOTH`, `BLUETOOTH_ADMIN`,
  `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, `ACCESS_FINE_LOCATION`.
- `uses-feature android.hardware.bluetooth_le` required.
- `application` label `@string/app_name` (`Zipp`), icon `@mipmap/ic_launcher`,
  `usesCleartextTraffic="true"` (the Entry URL is plain HTTP).
- Activity `.MainActivity`, exported, `singleTask`, `IcedTheme`,
  `configChanges` as in `pige`, `adjustResize`,
  `meta-data android.app.lib_name = "zipp"`.

Gradle:

- `settings.gradle.kts`: AGP 9.4.1, Kotlin 2.4.0, no Flutter plugin.
- `build.gradle.kts`: build directory redirected to `app/build/`.
- `app/build.gradle.kts`: namespace/applicationId `io.bitshift.zipp`,
  `compileSdk 36`, `minSdk 34`, `targetSdk 36`, NDK `28.2.13676358`, arm64
  only, `icedAndroidJava` source set, version name/code from Gradle properties,
  shared-key/debug signing fallback.
- `gradle.properties`, wrapper, `gradlew`, `.gitignore` copied from `pige`.
- `res/values/iced.xml`: black window background, transparent system bars,
  light bar icons false.
- `res/values/strings.xml`: `app_name = Zipp`.
- Reuse the existing `mipmap-*/ic_launcher.png` (the Flutter icon output).

---

## 9. Linux desktop

`zipp/src/platform/desktop.rs` runs a `btleplug` 0.13.5 actor on a dedicated
thread with a current-thread tokio runtime. It is chosen over `bluer` because it
is actively released (0.13.5, October 2026), well documented, and its Linux
backend still uses the maintained `bluez-async` over BlueZ; the `Transport`
trait keeps the choice isolated.

- Scan: `ScanFilter::default()`, `start_scan`, a 15 s window, a peripheral
  snapshot every 500 ms, names from `properties().local_name`.
- Connect: `peripheral.connect()`, `discover_services()`, subscribe to every
  notify/indicate characteristic, read every readable one, then forward a
  `notifications()` stream.
- Write: find the characteristic by UUID and use `WriteType::WithoutResponse`
  when the property allows it, otherwise `WithResponse`.
- Disconnect: `peripheral.disconnect()`; a notification stream ending while the
  peripheral is disconnected also emits `Disconnected`.
- Reconnect: scan (BlueZ paired devices are not exposed by `btleplug`).
- No adapter / adapter off: emit `AdapterState(false)` and log
  `Bluetooth is off`; never panic.
- GPS: `Disabled`.
- Wi-Fi: always `true`; Entry sends immediately.
- Entry HTTP: `reqwest` with default features off.

The window opens in `App::new` with
`window::Settings { size: Size::new(420.0, 820.0), min_size: Some(Size::new(360.0, 640.0)), .. }`
and is resizable. The dashboard keeps its fixed 480 dp height; the scanner takes
the rest.

### Lifecycle

- Android: the iced window is never closed on background. `Foreground(false)`
  does not close it; a stale background flag is cleared on
  `window::Event::Focused`, following qlaunch's rule. BLE/GPS keep running,
  exactly as the Flutter services did.
- Insets: `iced_android::insets()` -> `Message::Insets`; the root container pads
  by `Insets`. Desktop insets are zero.
- Scale: `iced_android::system_scale()` -> `Message::SystemScale`; the daemon's
  `scale_factor` uses it. Desktop is 1.0.
- System bars: `iced_android::set_system_bars_dark(true)` at startup.

---

## 10. Flutter removal and repository changes

### Delete

- `app/lib/`, `app/test/`, `app/integration_test/`, `app/test_driver/`.
- `app/ios/`, `app/linux/`, `app/macos/`, `app/windows/`, `app/web/`.
- `app/rust_builder/`.
- `app/pubspec.yaml`, `app/pubspec.lock`, `app/analysis_options.yaml`,
  `app/.metadata`, `app/flutter_rust_bridge.yaml`.
- `app/rust/src/frb_generated.rs`, `app/rust/src/api/`, the old
  `app/rust/Cargo.toml` and `app/rust/Cargo.lock` (workspace replaces them).
- Flutter Gradle/Kotlin/theme files under `app/android/` (replaced).

### Rewrite

- `app/README.md`: iced build, run, release, device log tag.
- Root `README.md`: change `app-java` to `app` and describe iced.
- `app/.gitignore`: Rust/Android ignores.
- `app/android/.gitignore`, Gradle files, manifest, resources.
- `app/build.sh`, `app/run.sh`.

### Add

- `app/ARCHITECTURE.md`: crates, data flow, transport trait, lifecycle.
- `app/docs/technical-notes.md`: build/run measurements, device results.
- `app/scripts/{build,release,version,publish}.sh`.
- `app/run_adb.sh`.
- Root `.gitignore` (the repo has none).

### Keep

- `app/assets/app_icon.{png,svg}`.
- `app/android/.../mipmap-*`.
- `app/android/gradle/`, `gradlew`, `gradlew.bat`.
- `old/`, `docs/`, `diagrams/`, `mcu-nrf-rs/`, `.git`.

---

## 11. Scripts

`app/scripts/build.sh` is `pige`'s script adapted:

- JDK detection (`/opt/jbr-21`, `/usr/lib/jvm/java-*-openjdk`, ...).
- `ANDROID_HOME`, `ANDROID_NDK_HOME` defaults and the 16 KiB page flag.
- `cargo build --locked -p zipp --lib --target aarch64-linux-android`.
- copy `libzipp.so` to `android/app/src/main/jniLibs/arm64-v8a/`.
- resolve `iced_android`'s Java source from `cargo metadata`.
- `VERSION` from `scripts/version.sh`, `VERSION_CODE` from
  `YYYYMMDD` arithmetic, pass `-PzippVersionName` and `-PzippVersionCode`.
- Gradle `assembleDebug` / `assembleRelease`.

`release.sh` builds the desktop `zipp` binary and the release APK into
`dist/zipp-<version>-linux-<arch>` and
`dist/zipp-<version>-android-arm64.apk`. `version.sh` reads the `zipp` package
version. `publish.sh` tags `v<version>` and creates the GitHub release with the
same artefacts. `run.sh` runs the desktop binary with the Wayland/XDG
environment filled in. `run_adb.sh` builds, installs, starts
`io.bitshift.zipp/.MainActivity`, and tails `adb logcat -s zipp`.

---

## 12. Testing and verification

### Automated

- `zipp_core` protocol tests: every UUID mapping, every command, empty/short
  data, unknown UUIDs, uppercase/lowercase, UART, four-digit expansion.
- `zipp_core` session tests: scan results and auto-connect, connect/disconnect,
  notification parsing, default state, log cap, clear, GPS conversion, entry
  decision, language selection.
- UI helper tests: 12-hour clock formatting, default-or-empty, indicator
  activity, button on/off state.
- `cargo test --workspace`.
- `cargo check --workspace --target aarch64-linux-android`.
- `cargo clippy --workspace --all-targets` (warnings accepted only if the
  reference projects have them).

### Manual — desktop

1. `./run.sh`; dashboard matches the Flutter layout.
2. Resize the window; the dashboard stays 480, the scanner fills.
3. Entry button sends a GET; the platform log shows the status or the connection
   error.
4. Scanner shows "No devices found" or a missing/off adapter message without
   crashing.
5. With a scooter powered on, scan/connect/notify/write work.

### Manual — Android

1. `bash scripts/run_adb.sh release`.
2. Permission dialog appears; grant BLE/location.
3. Scanner reports `Scanning...`, then auto-connects to `BScooter`.
4. Dashboard shows speed, battery, temperature, clock, odometer, cruise bars.
5. Every button writes; the physical scooter reacts.
6. Log page streams UART and manual UART input reaches the scooter.
7. GPS shows searching, then active speed.
8. Rotate, background/foreground, and return; the window survives.
9. `adb logcat -s zipp` has no Rust panic or JNI exception.

### Flutter-free gate

```sh
grep -RinE 'flutter|dart|pubspec|flutter_rust_bridge' \
  --exclude-dir=old --exclude-dir=.git --exclude-dir=target \
  /home/bronson/Projects/zipp/app
```

Only the design/plan prose may mention the historical removal; no build input
may reference Flutter.

---

## 13. Risks and mitigations

| Risk | Mitigation |
| :--- | :--- |
| iced/iced_android fork API drift | pin the exact tags; mirror `pige`/`qlaunch` |
| `btleplug`/BlueZ unavailable or adapter off | emit `AdapterState(false)`, log, never panic |
| Android BLE permission model changes | request on launch, report denial, test on API 34 device |
| JNI thread/global-ref mistakes | follow `pige`'s cached `JavaVM` + global class ref pattern |
| Desktop Chinese tofu | probe Noto CJK/WQY; record the result; bundle a font only if needed |
| Scan filter by device name misses devices | match Flutter's `withNames`; also match `platformName` on Android |
| Page slide complexity | `float` spike; fall back to instant and document |
| Cleartext Entry URL | `usesCleartextTraffic="true"` |
| Release signing | debug-key fallback; shared key only if `keystore.properties` exists |

---

## 14. Migration order

The tree should build at every step:

1. Create the workspace, `zipp_core`, protocol move and tests. The Flutter app
   still builds; nothing else changes.
2. Add the iced shell with the dashboard, scanner and log composed from the
   component kit, backed by a fake transport. Run on desktop.
3. Add the desktop `btleplug` transport, GPS-disabled state and Entry HTTP.
   Run on desktop.
4. Replace the Android shell with Gradle + `IcedActivity` + `ZippBridge` + JNI.
   Build and run on the phone.
5. Delete Flutter/Dart, the bridge, platform shells and Flutter tooling.
6. Rewrite the docs and scripts; run the Flutter-free grep; cut a release.

---

## 15. Decisions

- **Android BLE:** Kotlin `BluetoothLeScanner`/`BluetoothGatt` behind JNI,
  because it is the same stack `flutter_blue_plus` wrapped, and it matches the
  `pige`/`qlaunch` JNI pattern. The Rust core owns UUID parsing, state and
  command bytes.
- **Linux BLE:** `btleplug` 0.13.5, chosen over `bluer` for its current release
  and documentation. The Linux backend is the same BlueZ stack; the
  `Transport` trait isolates the decision.
- **Repository location:** port in place under `app/`, preserving
  `mcu-nrf-rs`, `docs`, `diagrams` and `old`.
- **Desktop scope:** UI + Entry + new BLE; GPS disabled, as before.
- **Single window:** `iced::daemon` with a 420x820 window opened in `App::new`.
- **Core/shell split:** `zipp_core` is platform-independent and testable;
  `zipp` owns iced and platform adapters.
- **History:** `old/` is kept; only the active Flutter layer is removed.
