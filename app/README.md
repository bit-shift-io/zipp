# Zipp

Zipp is a Rust + [iced](https://iced.rs) dashboard for the `BScooter` ebike
controller. It speaks the scooter's BLE protocol and renders two screens:

- **Dashboard** — the live readout (speed, battery voltage/current, light and
  alarm state, odometer, trip time) plus the controls (power, lights, horn,
  alarm, Entry).
- **Log** — a scrolling diagnostic log of every BLE command and notification,
  with a manual UART input.

The same codebase builds for **Linux desktop** (development, over BlueZ) and
**Android** (the shipping target). There is no cross-platform UI toolkit or FFI
bridge: Rust owns the protocol, the session state machine and the transport
policy, and the Kotlin layer only answers OS questions. See
[`ARCHITECTURE.md`](ARCHITECTURE.md) for the crate layout and data flow, and
[`docs/technical-notes.md`](docs/technical-notes.md) for build measurements and
on-device results.

## Requirements

- **Rust** (stable, edition 2024).
- **JDK 21** — a full JDK with `javac`; `/opt/jbr-21` is auto-detected if
  `JAVA_HOME` is unset.
- **Android SDK 36** and **NDK `28.2.13676358`** (`ANDROID_HOME` /
  `ANDROID_NDK_HOME`), plus the checked-in Gradle wrapper.
- **Linux desktop BLE** needs **BlueZ** (the `btleplug` backend) and a
  Wayland/X session.

## Commands

Run these from `app/` (the Rust tests run from `rust/`):

| Command | What it does |
| :--- | :--- |
| `./run.sh` | Run the desktop app. Extra args go to cargo, e.g. `./run.sh --release`. |
| `cargo test --workspace` | Run the Rust tests (from `rust/`). |
| `bash scripts/build.sh debug` | Build the arm64 Android APK (`debug` or `release`). |
| `./run_adb.sh` | Build, install, launch and tail logcat on a device. |
| `bash scripts/release.sh` | Build versioned artefacts into `dist/`. |
| `bash scripts/publish.sh` | Publish `dist/` as a GitHub release. |

## Entry and cleartext HTTP

The Entry control POSTs to the scooter gateway at
`http://192.168.1.3:1880/entry` (`ENTRY_URL` in `rust/zipp_core/src/entry.rs`).
On Android the manifest sets `android:usesCleartextTraffic="true"` so this
plain-HTTP request is permitted; without it the platform blocks the call.

## Logging

Both targets log through the `log` crate. On Android the app installs an
`android_logger` tagged **`zipp`**:

```sh
adb logcat -s zipp
```

On desktop, `env_logger` writes the same lines to stdout.

## Documentation

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — crates, the transport contract, the
  Android lifecycle, where to change what.
- [`docs/technical-notes.md`](docs/technical-notes.md) — build measurements,
  device results and accepted deviations.
