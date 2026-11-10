# Zipp architecture

A pure **iced + Rust** Android dashboard for the `BScooter`, with a Linux
desktop target used for development. There is no Flutter, Dart or bridge: the
old Dart tree was removed and must not be reintroduced.

The design rule running through the whole codebase is **the core is the source
of truth**. Rust — not the UI, and not Kotlin — owns the scooter protocol, the
session state machine and the transport policy. The iced UI renders that state
and sends mutations back; the Kotlin layer only answers OS questions and
performs OS actions.

---

## Crates and build

| Path | What it is |
| :--- | :--- |
| `rust/zipp_core` | Platform-independent core: protocol, the `ScooterSession` state machine, the `Transport` trait, i18n, GPS/Entry/log policy. Builds `rlib`. |
| `rust/zipp` | The iced UI (`App`, `Message`, `view`/`update`), the platform transports and the JNI-facing entry points. Builds `cdylib` (Android) + the `zipp` desktop binary. |
| `android/` | Gradle app, Kotlin OS adapter (`MainActivity`, `ZippBridge`), manifest (BLE/location permissions). |
| `scripts/build.sh` | arm64 Android build: cargo → `jniLibs` → Gradle. |
| `run.sh` / `run_adb.sh` | Run on desktop / build-install-launch on a device. |

Toolchain: Rust edition 2024, iced pinned to the `iced_android-v0.1.1` fork and
`iced_android v0.1.1`, Android NDK `28.2.13676358`, JDK 21 (`/opt/jbr-21`).

```sh
# Android release APK
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build.sh release
adb install -r build/app/outputs/apk/release/app-release.apk

# Desktop (needs a Wayland/X session)
./run.sh

# Tests and lints
cd rust && cargo test --workspace
cd rust && cargo clippy --workspace --all-targets
```

`cargo test --workspace` is the gate. On-device verification notes and
measurements are in `docs/technical-notes.md`.

---

## Data flow

`App` owns a `ScooterSession` and an `Arc<dyn Transport>`. Every interaction is
one Elm loop:

```text
button/input -> Message -> App::update -> session mutation + Task
  -> Transport::send -> platform actor (btleplug or JNI)
  -> TransportEvent -> broadcast -> subscription -> Message::Transport
  -> session.apply -> view
```

- A control sends a `Message`; `App::update` (`rust/zipp/src/update.rs`) mutates
  the session and forwards the returned `Vec<TransportCommand>` to the
  transport.
- The platform actor turns each event into a `TransportEvent` on a broadcast
  channel. `platform::transport_subscription()` maps them to
  `Message::Transport` and re-enters `session.apply`.
- `session.apply` parses notifications/reads into `ScooterState`, drives
  auto-connect and reconnection, and logs; the next `view` draws the new state.
- GPS updates carry their own channel (`Message::Gps`), and the Android Back
  button its own (`Message::Back`); neither round-trips through the session.

The only process-global state is the Android-side transport handle, the scan
snapshot and the last GPS status, all behind `OnceLock`/`Mutex`; the model
itself is owned by `App`. The transport handle is installed by `nativeInit`
before the window starts (see below).

`zipp` depends on `zipp_core`, never the reverse.

---

## Transport contract

`Transport` (`rust/zipp_core/src/transport.rs`) is the seam the whole platform
split hangs on:

```rust
pub trait Transport: Send + Sync + 'static {
    fn send(&self, command: TransportCommand);
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TransportEvent>;
    fn wifi(&self) -> tokio::sync::watch::Receiver<bool>;
}
```

- `TransportCommand` (`Reconnect`, `StartScan`, `StopScan`, `Connect`,
  `Disconnect`, `Write`, `Read`, `RequestPermissions`) is what the core asks the
  platform to do.
- `TransportEvent` (`AdapterState`, `ScanState`, `Devices`, `Connected`,
  `Disconnected`, `Notification`, `ReadValue`, `Wifi`, `Error`) is what the
  platform reports back.
- `DeviceId` wraps the Android MAC address or the `btleplug` peripheral id as a
  `String`, so the core never depends on either platform's id type.

Both platform adapters live behind the trait, so the core is testable without a
renderer or a radio:

| Target | Implementation |
| :--- | :--- |
| Linux | `rust/zipp/src/platform/desktop.rs` — a `btleplug` (BlueZ) actor on its own thread. |
| Android | `rust/zipp/src/platform/android.rs` + `jni.rs` — a broadcast sender out, static `ZippBridge` calls in. |
| Other / tests | `NoopTransport` (`platform/mod.rs`) and `FakeTransport` (test-only). |

**Why the split falls where it does.** The Android OS APIs — `BluetoothLeScanner`,
`BluetoothGatt`, `LocationManager`, `ConnectivityManager` and the runtime
permission flow — live in Kotlin because they *are* the platform's own API
surface. Everything that defines the scooter protocol instead lives in Rust:
UUID normalisation, `parse_characteristic_data`, `create_command_bytes` and the
command-to-service/characteristic mapping. That logic is identical on every
platform and is covered by the core's unit tests, so it must not be duplicated
in Kotlin. `ZippBridge` therefore only answers questions and performs actions;
it never interprets payloads.

The JNI symbol names (`Java_io_bitshift_zipp_ZippBridge_*`), the Kotlin
`external fun`s and the Rust `call_static_method` signatures are one contract;
keep the three in sync when adding a capability.

---

## Android lifecycle

- `JNI_OnLoad` caches the `JavaVM` and a global `ZippBridge` class reference on
  the thread that loaded the library, and installs the `zipp` logcat logger.
- `MainActivity.onCreate` calls `nativeInit` **before** `super.onCreate`. That
  installs the process-wide transport and the device locale, so the transport
  and language exist before `android_main` opens the iced window.
- `android_main` then calls `iced_android::init(app)` and `run()`.
- The app keeps **one** iced window. Backgrounding the activity does not
  deliver a window close request, and the app never returns `window::close`, so
  the window survives and BLE/GPS keep running while backgrounded. (Verified on
  device: Home then relaunch returned the same process and `ActivityRecord` with
  no destroy/recreate.)
- **Back** is dispatched through two paths that converge on `Message::Back`:
  `MainActivity.onBackPressed` → `ZippBridge.nativeBack` → the BACK broadcast →
  `back_subscription`, and the iced keyboard subscription mapping
  `BrowserBack`/`Escape`. The log page closes on either.
- Insets and scale arrive as `iced_android::insets()` /
  `iced_android::system_scale()` subscriptions; `set_system_bars_dark(true)` is
  set once at startup.

---

## Page and state ownership

`ScooterSession` (`rust/zipp_core/src/session.rs`) is pure: no I/O, and the
current time is passed in. It owns the parsed `ScooterState`, the scan list and
connection flags, `adapter_on`, the `LogBuffer`, the GPS status/speed, the
pending-Entry flag, the language and the target name. Its transitions —
`begin_scan`, `connect`, `send_command`, `tick`, `apply` — return the
`TransportCommand`s the UI must send, or nothing.

`App` (`rust/zipp/src/app.rs`) owns everything that is UI, not model: the
current `Page` (Dashboard / Log), the UART input string, the Android `insets`
and `scale`, the current time, and the `mobile` flag that selects the Entry
policy. It also holds the `Arc<dyn Transport>`.

The two pages are Dashboard and Log. The only deliberate UI deviation is that
the switch between them is an **instant swap** rather than Flutter's 300 ms
slide; every other layout, colour, size and interaction is preserved (the
accepted approximations are listed in `docs/technical-notes.md`).

---

## Where to change what

| Task | Start here |
| :--- | :--- |
| Scooter protocol / UUID mapping | `zipp_core/src/protocol.rs`; the session and UI must not re-derive it. |
| Session behaviour (scan, connect, logs, Entry) | `zipp_core/src/session.rs`; add a test next to the pure transition. |
| A new control or screen | `zipp/src/view.rs` and `zipp/src/components/`, with a `Message` variant and an `update.rs` arm. |
| A colour, size or radius | `zipp/src/theme.rs` only. No module invents one-off styling. |
| Desktop transport | `zipp/src/platform/desktop.rs`. |
| Android transport / OS action | `zipp/src/platform/android.rs` + `zipp/src/jni.rs`, then `android/.../ZippBridge.kt`. |

---

## Testing

- `cd rust && cargo test --workspace` — unit tests live next to the code.
- Geometry and pure state (protocol, session, i18n, GPS, log, view helpers) are
  deliberately testable without a renderer or a radio. Keep new logic pure and
  covered.
- `cargo check -p zipp --target aarch64-linux-android` exercises the non-Linux
  (no `btleplug`) build.
- `cargo fmt --all --check` and
  `cargo clippy -p zipp_core -p zipp --all-targets -- -D warnings` are clean.
