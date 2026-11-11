# Zipp — Flutter to iced port — Implementation Plan

> **For the implementer:** Work one task at a time. Each task is a vertical slice
> with a failing test first where the code is testable. Run the commands exactly
> as written, read the output, then commit. Do not jump ahead to the Android
> shell before the desktop shell and transport run.

**Date:** 2026-11-08
**Spec:** `app/docs/superpowers/specs/2026-11-08-zipp-iced-port-design.md`
**Repository:** `/home/bronson/Projects/zipp`
**App root:** `/home/bronson/Projects/zipp/app`
**Reference shells:** `/home/bronson/Projects/pige` and
`/home/bronson/Projects/qlaunch`

**Goal:** Replace the Flutter UI with a Rust + iced app that keeps the existing
scooter protocol, dashboard, scanner, log page, GPS and Entry behaviour on Linux
and Android, then delete Flutter from the active tree.

**Architecture:** A two-crate workspace at `app/rust`. `zipp_core` is
platform-independent and owns `ScooterState`, parsing, commands, the transport
contract, session transitions, i18n, GPS maths, Entry policy and the log buffer.
`zipp` owns iced, the desktop binary, platform transports and the Android JNI
exports. Android BLE/GPS/connectivity live in the Kotlin `ZippBridge`; Linux BLE
uses `btleplug`.

**Stack:** Rust edition 2024, iced pinned to `Santuzius/iced` tag
`iced_android-v0.1.1`, `iced_android` tag `v0.1.1`, Android NDK
`28.2.13676358`, JDK `/opt/jbr-21`, minSdk 34, targetSdk 36, arm64 only.

**Migration rule:** Until Task 19 the old Flutter package stays in
`app/rust/Cargo.toml` as the workspace's root package, so the Flutter build keeps
working. Tasks 1–18 add the new crates beside it. Task 19 removes the root
package and the last Flutter files.

---

## File map

```text
app/
  rust/
    Cargo.toml                    # workspace (Task 1); pure workspace (Task 19)
    zipp_core/
      Cargo.toml
      src/lib.rs
      src/protocol.rs
      src/transport.rs
      src/session.rs
      src/i18n.rs
      src/gps.rs
      src/entry.rs
      src/log.rs
    zipp/
      Cargo.toml
      assets/MaterialIcons-Regular.otf
      assets/MaterialIcons_LICENSE.txt
      src/lib.rs
      src/main.rs
      src/app.rs
      src/message.rs
      src/update.rs
      src/view.rs
      src/theme.rs
      src/fonts.rs
      src/icons.rs
      src/jni.rs
      src/components/{mod,button,stat,scanner,log,chrome}.rs
      src/platform/{mod,desktop,android}.rs
  android/
    app/src/main/AndroidManifest.xml
    app/src/main/kotlin/io/bitshift/zipp/{MainActivity.kt,ZippBridge.kt}
    app/src/main/res/values/{iced.xml,strings.xml}
  scripts/{build,release,version,publish}.sh
  ARCHITECTURE.md
  docs/technical-notes.md
```

**Conventions**

- All commands run from `/home/bronson/Projects/zipp` unless a task says
  otherwise.
- `cargo fmt --all` after every Rust edit.
- Commit messages use the imperative mood and one task per commit.
- Keep every user-facing string in `zipp_core::i18n`; never hardcode a label in
  `zipp`.
- Keep every colour, size and radius in `zipp::theme`; components never invent
  values.

---

## Task 1: Create the workspace and move the protocol core

**Files**

- Modify: `app/rust/Cargo.toml` (keep the Flutter `[package]`, add `[workspace]`)
- Create: `app/rust/zipp_core/Cargo.toml`
- Create: `app/rust/zipp_core/src/lib.rs`
- Create: `app/rust/zipp_core/src/protocol.rs`
- Keep: `app/rust/src/api/protocol.rs` untouched until Task 19

**Step 1: Extend the manifest**

Replace `app/rust/Cargo.toml` with:

```toml
[package]
name = "rust_lib_zipp"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
flutter_rust_bridge = "=2.12.0"
anyhow = "1.0"
log = "0.4"
serde = { version = "1.0", features = ["derive"] }

[workspace]
resolver = "2"
members = ["zipp_core"]

[workspace.package]
version = "2026.11.8"

[patch.crates-io]
android-activity = { git = "https://github.com/Santuzius/android-activity", tag = "iced_android-v0.1.1" }

[profile.release]
strip = "debuginfo"
```

Create `app/rust/zipp_core/Cargo.toml`:

```toml
[package]
name = "zipp_core"
version.workspace = true
edition = "2024"

[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
thiserror = "2"
tokio = { version = "1", features = ["sync"] }
```

Create `app/rust/zipp_core/src/lib.rs`:

```rust
//! Platform-independent Zipp core: protocol, model, session and policy.

pub mod protocol;
```

**Step 2: Write the failing protocol tests**

Create `app/rust/zipp_core/src/protocol.rs` with the types and empty/unimplemented
bodies, then the tests below. The tests must fail to compile first only because
the functions are missing; add the signatures immediately so the failure is a
test assertion, not a syntax error.

Public API:

```rust
use std::fmt;

pub const TARGET_DEVICE_NAME: &str = "BScooter";

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ScooterState {
    pub power_on: bool,
    pub alarm_on: bool,
    pub sport_on: bool,
    pub lights_on: bool,
    pub horn_active: bool,
    pub speed: String,
    pub throttle_level: String,
    pub temperature: String,
    pub clock_minutes: String,
    pub clock_hours: String,
    pub cruise_level: i32,
    pub battery_level: String,
    pub battery_power: String,
    pub brake_active: bool,
    pub park_brake_active: bool,
    pub odometer: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScooterCommand {
    TogglePower,
    ToggleAlarm,
    ToggleSport,
    ToggleLights,
    CruiseUp,
    CruiseDown,
    Horn,
    SetUart(String),
}

impl fmt::Display for ScooterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::TogglePower => "TogglePower",
            Self::ToggleAlarm => "ToggleAlarm",
            Self::ToggleSport => "ToggleSport",
            Self::ToggleLights => "ToggleLights",
            Self::CruiseUp => "CruiseUp",
            Self::CruiseDown => "CruiseDown",
            Self::Horn => "Horn",
            Self::SetUart(_) => "SetUart",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BleAction {
    pub bytes: Vec<u8>,
    pub service_uuid: String,
    pub characteristic_uuid: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub state: ScooterState,
    pub log: Option<String>,
}

pub fn target_device_name() -> String {
    TARGET_DEVICE_NAME.to_string()
}

pub fn parse_characteristic_data(
    state: ScooterState,
    uuid: &str,
    data: &[u8],
) -> ParseResult;

pub fn create_command_bytes(
    command: &ScooterCommand,
    current_state: &ScooterState,
) -> Vec<u8>;

pub fn get_command_action(
    command: &ScooterCommand,
    current_state: &ScooterState,
) -> BleAction;
```

Tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ScooterState {
        ScooterState::default()
    }

    #[test]
    fn target_is_bscooter() {
        assert_eq!(target_device_name(), "BScooter");
    }

    #[test]
    fn standard_uuids_map_to_their_four_hex_digits() {
        let uuid = "00002001-0000-1000-8000-00805F9B34FB";
        let parsed = parse_characteristic_data(state(), uuid, &[42]);
        assert_eq!(parsed.state.speed, "42");
    }

    #[test]
    fn custom_uart_uuid_is_kept_whole_and_lowercased() {
        let uuid = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E";
        let parsed = parse_characteristic_data(state(), uuid, b"hello");
        assert_eq!(parsed.log.as_deref(), Some("hello"));
    }

    #[test]
    fn short_uuid_0003_is_uart_too() {
        let parsed = parse_characteristic_data(state(), "0003", b"hi");
        assert_eq!(parsed.log.as_deref(), Some("hi"));
    }

    #[test]
    fn settings_characteristics_update_booleans() {
        let mut s = state();
        s = parse_characteristic_data(s, "1001", &[1]).state;
        s = parse_characteristic_data(s, "1003", &[1]).state;
        s = parse_characteristic_data(s, "1004", &[1]).state;
        s = parse_characteristic_data(s, "1005", &[1]).state;
        assert!(s.power_on && s.lights_on && s.alarm_on && s.sport_on);
        s = parse_characteristic_data(s, "1001", &[0]).state;
        assert!(!s.power_on);
    }

    #[test]
    fn data_characteristics_match_the_flutter_mapping() {
        let mut s = state();
        s = parse_characteristic_data(s, "2001", &[42]).state;
        s = parse_characteristic_data(s, "2002", &[0x34, 0x12]).state;
        s = parse_characteristic_data(s, "2004", &[7]).state;
        s = parse_characteristic_data(s, "2005", &[9]).state;
        s = parse_characteristic_data(s, "2006", &[13]).state;
        s = parse_characteristic_data(s, "2007", &[1]).state;
        s = parse_characteristic_data(s, "2008", &[1]).state;
        s = parse_characteristic_data(s, "2009", &[3]).state;
        s = parse_characteristic_data(s, "2a19", &[88]).state;
        s = parse_characteristic_data(s, "2b05", &[0x10, 0x27]).state;
        s = parse_characteristic_data(s, "2003", &[0x39, 0x30]).state;
        assert_eq!(s.speed, "42");
        assert_eq!(s.throttle_level, "4660");
        assert_eq!(s.temperature, "07");
        assert_eq!(s.clock_minutes, "09");
        assert_eq!(s.clock_hours, "13");
        assert!(s.brake_active && s.park_brake_active);
        assert_eq!(s.cruise_level, 3);
        assert_eq!(s.battery_level, "88");
        assert_eq!(s.battery_power, "10000");
        assert_eq!(s.odometer, "12345");
    }

    #[test]
    fn empty_and_short_data_leave_state_alone() {
        let before = state();
        for (uuid, data) in [
            ("1001", &[][..]),
            ("2001", &[][..]),
            ("2002", &[1][..]),
            ("2b05", &[1][..]),
            ("2003", &[1][..]),
        ] {
            let after = parse_characteristic_data(before.clone(), uuid, data);
            assert_eq!(after.state, before, "{uuid} changed on short data");
        }
    }

    #[test]
    fn unknown_uuids_and_invalid_utf8_are_ignored() {
        let parsed = parse_characteristic_data(state(), "ffff", &[1, 2, 3]);
        assert_eq!(parsed.state, state());
        assert!(parsed.log.is_none());
        let parsed = parse_characteristic_data(state(), "0003", &[0xff, 0xfe]);
        assert!(parsed.log.is_none());
    }

    #[test]
    fn uppercase_and_lowercase_standard_uuids_are_equivalent() {
        let upper = parse_characteristic_data(state(), "00002001-0000-1000-8000-00805F9B34FB", &[9]);
        let lower = parse_characteristic_data(state(), "00002001-0000-1000-8000-00805f9b34fb", &[9]);
        assert_eq!(upper.state.speed, "09");
        assert_eq!(lower.state.speed, "09");
    }

    #[test]
    fn command_bytes_are_exact() {
        let mut s = state();
        assert_eq!(create_command_bytes(&ScooterCommand::TogglePower, &s), vec![183]);
        s.power_on = true;
        assert_eq!(create_command_bytes(&ScooterCommand::TogglePower, &s), vec![0]);

        let mut s = state();
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleAlarm, &s), vec![205]);
        s.alarm_on = true;
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleAlarm, &s), vec![0]);

        let mut s = state();
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleSport, &s), vec![1]);
        s.sport_on = true;
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleSport, &s), vec![0]);

        let mut s = state();
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleLights, &s), vec![1]);
        s.lights_on = true;
        assert_eq!(create_command_bytes(&ScooterCommand::ToggleLights, &s), vec![0]);

        assert_eq!(create_command_bytes(&ScooterCommand::CruiseUp, &s), vec![1]);
        assert_eq!(create_command_bytes(&ScooterCommand::CruiseDown, &s), vec![0]);
        assert_eq!(create_command_bytes(&ScooterCommand::Horn, &s), vec![1]);
        assert_eq!(
            create_command_bytes(&ScooterCommand::SetUart("abc".into()), &s),
            b"abc".to_vec()
        );
    }

    #[test]
    fn command_actions_use_exact_services_and_expand_short_uuids() {
        let s = state();
        let cases = [
            (ScooterCommand::TogglePower, "00001000-0000-1000-8000-00805F9B34FB", "00001001-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::ToggleLights, "00001000-0000-1000-8000-00805F9B34FB", "00001003-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::ToggleAlarm, "00001000-0000-1000-8000-00805F9B34FB", "00001004-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::ToggleSport, "00001000-0000-1000-8000-00805F9B34FB", "00001005-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::CruiseUp, "00001000-0000-1000-8000-00805F9B34FB", "00001006-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::CruiseDown, "00001000-0000-1000-8000-00805F9B34FB", "00001006-0000-1000-8000-00805F9B34FB"),
            (ScooterCommand::Horn, "00001000-0000-1000-8000-00805F9B34FB", "00001002-0000-1000-8000-00805F9B34FB"),
        ];
        for (command, service, characteristic) in cases {
            let action = get_command_action(&command, &s);
            assert_eq!(action.service_uuid, service, "{command}");
            assert_eq!(action.characteristic_uuid, characteristic, "{command}");
        }
        let uart = get_command_action(&ScooterCommand::SetUart("x".into()), &s);
        assert_eq!(uart.service_uuid, "6e400001-b5a3-f393-e0a9-e50e24dcca9e");
        assert_eq!(uart.characteristic_uuid, "6e400002-b5a3-f393-e0a9-e50e24dcca9e");
    }

    #[test]
    fn power_action_expands_service_and_characteristic() {
        let action = get_command_action(&ScooterCommand::TogglePower, &state());
        assert_eq!(action.service_uuid, "00001000-0000-1000-8000-00805F9B34FB");
        assert_eq!(action.characteristic_uuid, "00001001-0000-1000-8000-00805F9B34FB");
    }
}
```

**Step 3: Run the tests**

```sh
cd app/rust
cargo test -p zipp_core
```

They fail until the bodies are implemented.

**Step 4: Implement the module**

Copy the behaviour of `app/rust/src/api/protocol.rs` exactly:

- UUID normalisation: a 36-character UUID is lowercased; if it starts with
  `0000` and ends with `-0000-1000-8000-00805f9b34fb`, keep only bytes 4..8;
  otherwise keep the whole lowercased UUID. Any other length is lowercased.
- Mapping: `1001` power, `1003` lights, `1004` alarm, `1005` sport, `2001` speed
  (`{:02}`), `2002` throttle (LE u16), `2003` odometer (LE u16, `{:04}`), `2004`
  temperature (`{:02}`), `2005` clock minutes (`{:02}`), `2006` clock hours
  (`{:02}`), `2007` brake, `2008` park brake, `2009` cruise level, `2a19`
  battery level, `2b05` battery power (LE u16), `0003` and the lowercased
  `6e400003-...` UART RX.
- `create_command_bytes` uses Power off 0 / on 183, Alarm off 0 / on 205, Sport
  off 0 / on 1, Lights off 0 / on 1, CruiseUp 1, CruiseDown 0, Horn 1, SetUart
  UTF-8 bytes.
- `get_command_action` uses the service/characteristic pairs in the test and
  expands a four-character UUID with
  `format!("0000{}-0000-1000-8000-00805F9B34FB", short)`. Custom UART UUIDs stay
  lowercase.

`parse_characteristic_data` takes `ScooterState` by value and returns a new one,
so the borrow checker stays simple.

**Step 5: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp_core
cd ../..
git add app/rust/Cargo.toml app/rust/Cargo.lock app/rust/zipp_core
git commit -m "feat(core): add workspace and move scooter protocol"
```

---

## Task 2: Add the transport contract and the log buffer

**Files**

- Create: `app/rust/zipp_core/src/transport.rs`
- Create: `app/rust/zipp_core/src/log.rs`
- Modify: `app/rust/zipp_core/src/lib.rs`

**Step 1: Write the failing tests**

In `log.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn at(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    #[test]
    fn formats_hour_and_minute_before_the_message() {
        let mut log = LogBuffer::new();
        log.push("start scan", at(9, 5));
        assert_eq!(log.lines().next().unwrap(), "09:05 start scan");
    }

    #[test]
    fn empty_messages_keep_the_timestamp_and_a_space() {
        let mut log = LogBuffer::new();
        log.push("", at(23, 59));
        assert_eq!(log.lines().next().unwrap(), "23:59 ");
    }

    #[test]
    fn keeps_only_the_newest_two_hundred_lines() {
        let mut log = LogBuffer::new();
        for index in 0..205 {
            log.push(index.to_string(), at(1, 0));
        }
        assert_eq!(log.len(), MAX_LOGS);
        assert_eq!(log.lines().next().unwrap(), "01:00 5");
    }

    #[test]
    fn clear_empties_the_buffer() {
        let mut log = LogBuffer::new();
        log.push("x", at(1, 1));
        log.clear();
        assert!(log.is_empty());
    }
}
```

In `transport.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_devices_are_ordered_and_comparable() {
        let a = ScanDevice { id: DeviceId("AA".into()), name: "BScooter".into(), rssi: Some(-40) };
        let b = ScanDevice { id: DeviceId("BB".into()), name: "Other".into(), rssi: None };
        assert_ne!(a, b);
        assert_eq!(a.id.0, "AA");
    }

    #[test]
    fn commands_and_events_round_trip_their_payloads() {
        let command = TransportCommand::Write {
            service: "s".into(),
            characteristic: "c".into(),
            bytes: vec![1, 2],
        };
        assert!(matches!(command, TransportCommand::Write { bytes, .. } if bytes == vec![1, 2]));
        let event = TransportEvent::Connected { id: DeviceId("AA".into()), name: "BScooter".into() };
        assert!(matches!(event, TransportEvent::Connected { name, .. } if name == "BScooter"));
    }
}
```

**Step 2: Run the tests and watch them fail**

```sh
cd app/rust
cargo test -p zipp_core log:: transport::
```

**Step 3: Implement**

`transport.rs`:

```rust
use tokio::sync::{broadcast, watch};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDevice {
    pub id: DeviceId,
    pub name: String,
    pub rssi: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportCommand {
    /// Android checks bonded devices first, then scans. Desktop scans.
    Reconnect { target: String },
    StartScan { target: String },
    StopScan,
    Connect { id: DeviceId },
    Disconnect,
    Write {
        service: String,
        characteristic: String,
        bytes: Vec<u8>,
    },
    Read {
        service: String,
        characteristic: String,
    },
    RequestPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub trait Transport: Send + Sync + 'static {
    fn send(&self, command: TransportCommand);
    fn subscribe(&self) -> broadcast::Receiver<TransportEvent>;
    fn wifi(&self) -> watch::Receiver<bool>;
}
```

`log.rs`:

```rust
use std::collections::VecDeque;

use chrono::{Local, NaiveTime};

pub const MAX_LOGS: usize = 200;

#[derive(Debug, Clone, Default)]
pub struct LogBuffer {
    lines: VecDeque<String>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: impl AsRef<str>, at: NaiveTime) {
        self.lines.push_back(format!(
            "{:02}:{:02} {}",
            at.format("%H"),
            at.format("%M"),
            message.as_ref()
        ));
        while self.lines.len() > MAX_LOGS {
            self.lines.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> &VecDeque<String> {
        &self.lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

pub fn now() -> NaiveTime {
    Local::now().time()
}
```

Use `at.format("%H")`/`%M` so the output is always zero-padded and testable
without a timezone. Add `pub mod log; pub mod transport;` to `lib.rs`.

**Step 4: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp_core
cd ../..
git add app/rust/zipp_core
git commit -m "feat(core): add transport contract and log buffer"
```

---

## Task 3: Add GPS and Entry policy

**Files**

- Create: `app/rust/zipp_core/src/gps.rs`
- Create: `app/rust/zipp_core/src/entry.rs`
- Modify: `app/rust/zipp_core/src/lib.rs`

**Step 1: Write the failing tests**

`gps.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_defaults_to_disabled() {
        assert_eq!(GpsStatus::default(), GpsStatus::Disabled);
    }

    #[test]
    fn converts_metres_per_second_to_kmh() {
        assert!((mps_to_kmh(1.0) - 3.6).abs() < f64::EPSILON);
        assert!((mps_to_kmh(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((mps_to_kmh(10.0) - 36.0).abs() < 1e-9);
    }

    #[test]
    fn an_update_with_a_speed_reports_kmh() {
        let update = GpsUpdate { status: GpsStatus::Active, speed_mps: Some(2.5) };
        assert!((update.speed_kmh().unwrap() - 9.0).abs() < 1e-9);
        assert_eq!(GpsUpdate { status: GpsStatus::Searching, speed_mps: None }.speed_kmh(), None);
    }
}
```

`entry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_always_sends_immediately() {
        assert_eq!(decide(false, false, false), EntryDecision::SendNow);
        assert_eq!(decide(false, true, false), EntryDecision::SendNow);
    }

    #[test]
    fn mobile_with_wifi_sends_immediately() {
        assert_eq!(decide(true, true, false), EntryDecision::SendNow);
    }

    #[test]
    fn mobile_without_wifi_waits() {
        assert_eq!(decide(true, false, false), EntryDecision::WaitForWifi);
    }

    #[test]
    fn an_in_flight_request_is_deduplicated() {
        assert_eq!(decide(false, true, true), EntryDecision::Ignore);
        assert_eq!(decide(true, true, true), EntryDecision::Ignore);
        assert_eq!(decide(true, false, true), EntryDecision::Ignore);
    }

    #[test]
    fn the_url_matches_flutter() {
        assert_eq!(ENTRY_URL, "http://192.168.1.3:1880/entry");
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp_core gps:: entry::
```

**Step 3: Implement**

`gps.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpsStatus {
    #[default]
    Disabled,
    Denied,
    Searching,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpsUpdate {
    pub status: GpsStatus,
    pub speed_mps: Option<f64>,
}

impl GpsUpdate {
    pub fn speed_kmh(&self) -> Option<f64> {
        self.speed_mps.map(mps_to_kmh)
    }
}

pub fn mps_to_kmh(mps: f64) -> f64 {
    mps * 3.6
}
```

`entry.rs`:

```rust
pub const ENTRY_URL: &str = "http://192.168.1.3:1880/entry";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryDecision {
    SendNow,
    WaitForWifi,
    Ignore,
}

pub fn decide(mobile: bool, wifi: bool, pending: bool) -> EntryDecision {
    if pending {
        EntryDecision::Ignore
    } else if !mobile || wifi {
        EntryDecision::SendNow
    } else {
        EntryDecision::WaitForWifi
    }
}
```

Add `pub mod entry; pub mod gps;` to `lib.rs`.

**Step 4: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp_core
cd ../..
git add app/rust/zipp_core
git commit -m "feat(core): add GPS and Entry policy"
```

---

## Task 4: Add English and Chinese strings

**Files**

- Create: `app/rust/zipp_core/src/i18n.rs`
- Modify: `app/rust/zipp_core/src/lib.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_default() {
        assert_eq!(Lang::from_tag("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::from_tag("de_DE"), Lang::En);
        assert_eq!(Lang::from_tag(""), Lang::En);
    }

    #[test]
    fn any_zh_tag_selects_chinese() {
        for tag in ["zh", "zh_CN.UTF-8", "ZH_TW", "zh-Hans"] {
            assert_eq!(Lang::from_tag(tag), Lang::Zh, "{tag}");
        }
    }

    #[test]
    fn strings_match_the_flutter_values() {
        let en = Lang::En.strings();
        assert_eq!(en.power, "Power");
        assert_eq!(en.cruise_up, "Cruise\nUp");
        assert_eq!(en.on, "ON");
        assert_eq!(en.clear, "clear");
        assert_eq!(en.scanner_title, "Log");
        let zh = Lang::Zh.strings();
        assert_eq!(zh.power, "电源");
        assert_eq!(zh.cruise_up, "巡航\n加速");
        assert_eq!(zh.on, "开");
        assert_eq!(zh.clear, "清除");
        assert_eq!(zh.scanner_title, "日志");
    }

    #[test]
    fn every_string_is_present_in_both_languages() {
        for strings in [Lang::En.strings(), Lang::Zh.strings()] {
            for value in strings.all() {
                assert!(!value.is_empty());
            }
        }
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp_core i18n::
```

**Step 3: Implement**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strings {
    pub power: &'static str,
    pub alarm: &'static str,
    pub lights: &'static str,
    pub sport: &'static str,
    pub cruise: &'static str,
    pub park: &'static str,
    pub brake: &'static str,
    pub horn: &'static str,
    pub entry: &'static str,
    pub cruise_up: &'static str,
    pub cruise_down: &'static str,
    pub kmh: &'static str,
    pub km: &'static str,
    pub celsius: &'static str,
    pub mv: &'static str,
    pub watts: &'static str,
    pub on: &'static str,
    pub off: &'static str,
    pub scanner_title: &'static str,
    pub scanning: &'static str,
    pub connecting: &'static str,
    pub disconnected: &'static str,
    pub connect: &'static str,
    pub disconnect: &'static str,
    pub unknown: &'static str,
    pub nearby: &'static str,
    pub log: &'static str,
    pub clear: &'static str,
}

impl Strings {
    pub fn all(&self) -> [&'static str; 28] {
        [
            self.power, self.alarm, self.lights, self.sport, self.cruise, self.park,
            self.brake, self.horn, self.entry, self.cruise_up, self.cruise_down,
            self.kmh, self.km, self.celsius, self.mv, self.watts, self.on, self.off,
            self.scanner_title, self.scanning, self.connecting, self.disconnected,
            self.connect, self.disconnect, self.unknown, self.nearby, self.log,
            self.clear,
        ]
    }
}

pub const EN: Strings = Strings {
    power: "Power",
    alarm: "Alarm",
    lights: "Lights",
    sport: "Sport",
    cruise: "Cruise",
    park: "Park",
    brake: "Brake",
    horn: "Horn",
    entry: "Entry",
    cruise_up: "Cruise\nUp",
    cruise_down: "Cruise\nDown",
    kmh: "km/h",
    km: "km",
    celsius: "°C",
    mv: "mv",
    watts: "w",
    on: "ON",
    off: "OFF",
    scanner_title: "Log",
    scanning: "Scanning...",
    connecting: "Connecting...",
    disconnected: "Disconnected",
    connect: "Connect",
    disconnect: "Disconnect",
    unknown: "Unknown Device",
    nearby: "Nearby Devices",
    log: "Log",
    clear: "clear",
};

pub const ZH: Strings = Strings {
    power: "电源",
    alarm: "报警器",
    lights: "车灯",
    sport: "运动模式",
    cruise: "定速巡航",
    park: "驻车",
    brake: "刹车",
    horn: "喇叭",
    entry: "门禁",
    cruise_up: "巡航\n加速",
    cruise_down: "巡航\n减速",
    kmh: "km/h",
    km: "km",
    celsius: "°C",
    mv: "mv",
    watts: "w",
    on: "开",
    off: "关",
    scanner_title: "日志",
    scanning: "扫描中...",
    connecting: "连接中...",
    disconnected: "已断开",
    connect: "连接",
    disconnect: "断开",
    unknown: "未知设备",
    nearby: "附近设备",
    log: "日志",
    clear: "清除",
};

impl Lang {
    pub const fn strings(self) -> &'static Strings {
        match self {
            Self::En => &EN,
            Self::Zh => &ZH,
        }
    }

    pub fn from_tag(tag: &str) -> Self {
        if tag.to_ascii_lowercase().starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }
}

/// Desktop detection: `LC_ALL`, then `LC_MESSAGES`, then `LANG`.
pub fn detect() -> Lang {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Lang::from_tag(&value);
            }
        }
    }
    Lang::En
}
```

Add `pub mod i18n;` to `lib.rs`.

**Step 4: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp_core
cd ../..
git add app/rust/zipp_core
git commit -m "feat(core): add English and Chinese strings"
```

---

## Task 5: Implement the pure scooter session

**Files**

- Create: `app/rust/zipp_core/src/session.rs`
- Modify: `app/rust/zipp_core/src/lib.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn now() -> NaiveTime {
        NaiveTime::from_hms_opt(9, 5, 0).unwrap()
    }

    fn device(name: &str) -> ScanDevice {
        ScanDevice { id: DeviceId(name.into()), name: name.into(), rssi: Some(-40) }
    }

    #[test]
    fn starts_disconnected_and_without_a_state() {
        let s = ScooterSession::new(Lang::En);
        assert!(s.state.is_none());
        assert!(s.connected.is_none());
        assert!(!s.scanning);
        assert!(!s.connecting);
        // Optimistic until the platform reports otherwise: the first scan must
        // not be suppressed before AdapterState(true) arrives.
        assert!(s.adapter_on);
        assert!(s.logs.is_empty());
    }

    #[test]
    fn scan_state_logs_and_clears_results() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::Devices(vec![device("Other")]), now());
        assert_eq!(s.devices.len(), 1);
        s.apply(TransportEvent::ScanState(true), now());
        assert!(s.scanning);
        assert!(s.devices.is_empty());
        assert_eq!(s.logs.lines().next().unwrap(), "09:05 start scan");
        s.apply(TransportEvent::ScanState(false), now());
        assert!(!s.scanning);
    }

    #[test]
    fn auto_connects_to_the_target_once() {
        let mut s = ScooterSession::new(Lang::En);
        let commands = s.apply(
            TransportEvent::Devices(vec![device("Other"), device("BScooter")]),
            now(),
        );
        assert_eq!(commands, vec![TransportCommand::Connect { id: DeviceId("BScooter".into()) }]);
        assert!(s.connecting);
        assert_eq!(s.logs.lines().next_back().unwrap(), "09:05 connecting: BScooter");
        let second = s.apply(TransportEvent::Devices(vec![device("BScooter")]), now());
        assert!(second.is_empty());
    }

    #[test]
    fn connecting_from_the_list_stops_scanning_first() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::ScanState(true), now());
        let commands = s.connect(device("BScooter"), now());
        assert_eq!(
            commands,
            vec![
                TransportCommand::StopScan,
                TransportCommand::Connect { id: DeviceId("BScooter".into()) }
            ]
        );
        assert!(!s.scanning);
        assert!(s.connecting);
    }

    #[test]
    fn connected_and_disconnected_update_the_state_and_log() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Connected { id: DeviceId("BScooter".into()), name: "BScooter".into() },
            now(),
        );
        assert!(!s.connecting);
        assert_eq!(s.connected.as_ref().unwrap().name, "BScooter");
        assert_eq!(s.logs.lines().next_back().unwrap(), "09:05 connected: BScooter");
        s.apply(TransportEvent::Disconnected, now());
        assert!(s.connected.is_none());
        assert_eq!(s.logs.lines().next_back().unwrap(), "09:05 disconnected from device");
    }

    #[test]
    fn notifications_parse_uart_into_the_log_and_state() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Notification { uuid: "2001".into(), data: vec![12] },
            now(),
        );
        assert_eq!(s.state.as_ref().unwrap().speed, "12");
        s.apply(
            TransportEvent::Notification { uuid: "0003".into(), data: b"hello".to_vec() },
            now(),
        );
        assert_eq!(s.logs.lines().next_back().unwrap(), "09:05 hello");
    }

    #[test]
    fn reads_update_state_without_logging() {
        let mut s = ScooterSession::new(Lang::En);
        let before = s.logs.len();
        s.apply(
            TransportEvent::ReadValue { uuid: "2a19".into(), data: vec![77] },
            now(),
        );
        assert_eq!(s.state.as_ref().unwrap().battery_level, "77");
        assert_eq!(s.logs.len(), before);
    }

    #[test]
    fn adapter_off_logs_and_stops_scanning() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(TransportEvent::ScanState(true), now());
        s.apply(TransportEvent::AdapterState(false), now());
        assert!(!s.adapter_on);
        assert!(!s.scanning);
        assert_eq!(s.logs.lines().next_back().unwrap(), "09:05 Bluetooth is off");
    }

    #[test]
    fn tick_reconnects_only_when_idle() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(
            s.tick(now()),
            vec![TransportCommand::Reconnect { target: "BScooter".into() }]
        );
        s.apply(TransportEvent::ScanState(true), now());
        assert!(s.tick(now()).is_empty());
        s.apply(TransportEvent::ScanState(false), now());
        s.apply(
            TransportEvent::Connected { id: DeviceId("BScooter".into()), name: "BScooter".into() },
            now(),
        );
        assert!(s.tick(now()).is_empty());
    }

    #[test]
    fn commands_log_the_flutter_text_and_write_the_action() {
        let mut s = ScooterSession::new(Lang::En);
        s.apply(
            TransportEvent::Connected { id: DeviceId("BScooter".into()), name: "BScooter".into() },
            now(),
        );
        s.logs.clear();
        let commands = s.send_command(ScooterCommand::TogglePower, now());
        assert_eq!(s.logs.lines().next().unwrap(), "09:05 > sending command: TogglePower");
        assert!(matches!(
            commands.as_slice(),
            [TransportCommand::Write { service, characteristic, bytes }]
                if service == "00001000-0000-1000-8000-00805F9B34FB"
                    && characteristic == "00001001-0000-1000-8000-00805F9B34FB"
                    && bytes == &vec![183]
        ));
    }

    #[test]
    fn uart_commands_add_an_empty_line_then_the_text() {
        let mut s = ScooterSession::new(Lang::En);
        let commands = s.send_command(ScooterCommand::SetUart("abc".into()), now());
        assert!(commands.is_empty());
        assert_eq!(s.logs.lines()[0], "09:05 ");
        assert_eq!(s.logs.lines()[1], "09:05 > abc");
        assert_eq!(s.logs.lines()[2], "09:05 Not connected");
    }

    #[test]
    fn entry_waits_only_on_mobile_without_wifi() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(s.request_entry(false, false), EntryDecision::SendNow);
        assert_eq!(s.request_entry(true, true), EntryDecision::SendNow);
        assert_eq!(s.request_entry(true, false), EntryDecision::WaitForWifi);
        assert!(s.entry_pending);
        assert_eq!(s.request_entry(true, true), EntryDecision::Ignore);
        s.finish_entry();
        assert!(!s.entry_pending);
    }

    #[test]
    fn gps_updates_store_kmh() {
        let mut s = ScooterSession::new(Lang::En);
        s.set_gps(GpsUpdate { status: GpsStatus::Active, speed_mps: Some(2.5) });
        assert_eq!(s.gps_status, GpsStatus::Active);
        assert!((s.gps_speed_kmh.unwrap() - 9.0).abs() < 1e-9);
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp_core session::
```

**Step 3: Implement**

```rust
use chrono::NaiveTime;

use crate::entry::{self, EntryDecision};
use crate::gps::{GpsStatus, GpsUpdate};
use crate::i18n::{Lang, Strings};
use crate::log::LogBuffer;
use crate::protocol::{
    BleAction, ScooterCommand, ScooterState, get_command_action, parse_characteristic_data,
    target_device_name,
};
use crate::transport::{
    DeviceId, ScanDevice, TransportCommand, TransportEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDevice {
    pub id: DeviceId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ScooterSession {
    pub state: Option<ScooterState>,
    pub devices: Vec<ScanDevice>,
    pub connected: Option<ConnectedDevice>,
    pub scanning: bool,
    pub connecting: bool,
    /// Optimistic `true` until the platform reports otherwise, so the first
    /// scan is not suppressed before `AdapterState(true)` arrives.
    pub adapter_on: bool,
    pub logs: LogBuffer,
    pub gps_status: GpsStatus,
    pub gps_speed_kmh: Option<f64>,
    pub entry_pending: bool,
    pub lang: Lang,
    pub target: String,
}

impl ScooterSession {
    pub fn new(lang: Lang) -> Self {
        Self {
            state: None,
            devices: Vec::new(),
            connected: None,
            scanning: false,
            connecting: false,
            adapter_on: true,
            logs: LogBuffer::new(),
            gps_status: GpsStatus::Disabled,
            gps_speed_kmh: None,
            entry_pending: false,
            lang,
            target: target_device_name(),
        }
    }

    pub fn strings(&self) -> &'static Strings {
        self.lang.strings()
    }

    pub fn log(&mut self, message: impl AsRef<str>, now: NaiveTime) {
        self.logs.push(message, now);
    }

    pub fn begin_scan(&mut self) -> Vec<TransportCommand> {
        if !self.adapter_on {
            return Vec::new();
        }
        vec![TransportCommand::StartScan { target: self.target.clone() }]
    }

    pub fn stop_scan(&mut self) -> Vec<TransportCommand> {
        vec![TransportCommand::StopScan]
    }

    pub fn connect(&mut self, device: ScanDevice, now: NaiveTime) -> Vec<TransportCommand> {
        if self.connecting {
            return Vec::new();
        }
        let mut commands = Vec::new();
        if self.scanning {
            self.scanning = false;
            self.log("stop scan", now);
            commands.push(TransportCommand::StopScan);
        }
        self.connecting = true;
        self.log(format!("connecting: {}", device.name), now);
        commands.push(TransportCommand::Connect { id: device.id });
        commands
    }

    pub fn disconnect(&mut self) -> Vec<TransportCommand> {
        vec![TransportCommand::Disconnect]
    }

    pub fn send_command(
        &mut self,
        command: ScooterCommand,
        now: NaiveTime,
    ) -> Vec<TransportCommand> {
        if matches!(command, ScooterCommand::SetUart(_)) {
            self.log("", now);
        }
        match &command {
            ScooterCommand::SetUart(text) => self.log(format!("> {text}"), now),
            other => self.log(format!("> sending command: {other}"), now),
        }
        let Some(connected) = self.connected.as_ref() else {
            self.log("Not connected", now);
            return Vec::new();
        };
        let state = self.state.clone().unwrap_or_default();
        let BleAction { bytes, service_uuid, characteristic_uuid } =
            get_command_action(&command, &state);
        let _ = connected;
        vec![TransportCommand::Write {
            service: service_uuid,
            characteristic: characteristic_uuid,
            bytes,
        }]
    }

    pub fn tick(&mut self, _now: NaiveTime) -> Vec<TransportCommand> {
        if self.connected.is_none() && !self.connecting && !self.scanning {
            vec![TransportCommand::Reconnect { target: self.target.clone() }]
        } else {
            Vec::new()
        }
    }

    pub fn apply(&mut self, event: TransportEvent, now: NaiveTime) -> Vec<TransportCommand> {
        match event {
            TransportEvent::AdapterState(on) => {
                self.adapter_on = on;
                if !on {
                    self.scanning = false;
                    self.log("Bluetooth is off", now);
                }
                Vec::new()
            }
            TransportEvent::ScanState(scanning) => {
                if scanning && !self.scanning {
                    self.devices.clear();
                    self.log("start scan", now);
                }
                self.scanning = scanning;
                Vec::new()
            }
            TransportEvent::Devices(devices) => {
                self.devices = devices;
                if self.connected.is_none() && !self.connecting {
                    if let Some(device) =
                        self.devices.iter().find(|d| d.name == self.target).cloned()
                    {
                        return self.connect(device, now);
                    }
                }
                Vec::new()
            }
            TransportEvent::Connected { id, name } => {
                self.connecting = false;
                self.scanning = false;
                self.log(format!("connected: {name}"), now);
                self.connected = Some(ConnectedDevice { id, name });
                Vec::new()
            }
            TransportEvent::Disconnected => {
                self.connecting = false;
                self.scanning = false;
                self.connected = None;
                self.log("disconnected from device", now);
                Vec::new()
            }
            TransportEvent::Notification { uuid, data } => {
                let state = self.state.clone().unwrap_or_default();
                let parsed = parse_characteristic_data(state, &uuid, &data);
                if let Some(line) = parsed.log {
                    self.log(line, now);
                }
                self.state = Some(parsed.state);
                Vec::new()
            }
            TransportEvent::ReadValue { uuid, data } => {
                let state = self.state.clone().unwrap_or_default();
                self.state = Some(parse_characteristic_data(state, &uuid, &data).state);
                Vec::new()
            }
            TransportEvent::Wifi(_) => Vec::new(),
            TransportEvent::Error(message) => {
                if self.connecting {
                    self.connecting = false;
                }
                self.log(message, now);
                Vec::new()
            }
        }
    }

    pub fn set_gps(&mut self, update: GpsUpdate) {
        self.gps_status = update.status;
        if let Some(kmh) = update.speed_kmh() {
            self.gps_speed_kmh = Some(kmh);
        }
    }

    pub fn request_entry(&mut self, mobile: bool, wifi: bool) -> EntryDecision {
        let decision = entry::decide(mobile, wifi, self.entry_pending);
        if decision == EntryDecision::WaitForWifi {
            self.entry_pending = true;
        }
        decision
    }

    pub fn finish_entry(&mut self) {
        self.entry_pending = false;
    }
}
```

While implementing, remove the `let Some(connected) = ...` binding if clippy
complains about the unused variable; the check only needs `is_none()`.

Add `pub mod session;` to `lib.rs`.

**Step 4: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp_core
cd ../..
git add app/rust/zipp_core
git commit -m "feat(core): add pure scooter session"
```

---

## Task 6: Create the iced crate, theme, fonts and icons

**Files**

- Modify: `app/rust/Cargo.toml` (add `zipp` to members)
- Create: `app/rust/zipp/Cargo.toml`
- Create: `app/rust/zipp/assets/MaterialIcons-Regular.otf`
- Create: `app/rust/zipp/assets/MaterialIcons_LICENSE.txt`
- Create: `app/rust/zipp/src/lib.rs`
- Create: `app/rust/zipp/src/main.rs`
- Create: `app/rust/zipp/src/theme.rs`
- Create: `app/rust/zipp/src/fonts.rs`
- Create: `app/rust/zipp/src/icons.rs`

**Step 1: Copy the font and license from the reference shell**

```sh
mkdir -p app/rust/zipp/assets
cp /home/bronson/Projects/pige/rust/pige/assets/MaterialIcons-Regular.otf \
   app/rust/zipp/assets/
cp /home/bronson/Projects/pige/rust/pige/assets/MaterialIcons_LICENSE.txt \
   app/rust/zipp/assets/
```

Verify the codepoints before writing `icons.rs`:

```sh
python3 - <<'PY'
from fontTools.ttLib import TTFont
font = TTFont('app/rust/zipp/assets/MaterialIcons-Regular.otf')
cmap = font.getBestCmap()
for name in ['gps_fixed','gps_not_fixed','bluetooth','bluetooth_connected',
             'bluetooth_disabled','bluetooth_searching','search','stop',
             'arrow_back_ios','chevron_right','add','close','arrow_back']:
    hits = [(cp, glyph) for cp, glyph in cmap.items()
            if glyph in (name, name + '_baseline')]
    print(name, ' '.join(f'U+{cp:04X}' for cp, _ in hits))
PY
```

Expected: `gps_fixed U+E2DC`, `gps_not_fixed U+E2DD`, `bluetooth U+E0E4`,
`bluetooth_connected U+E0E6`, `bluetooth_disabled U+E0E7`,
`bluetooth_searching U+E0E9`, `search U+E567`, `stop U+E606`,
`arrow_back_ios U+E093`, `chevron_right U+E15F`, `add U+E047`, `close U+E16A`,
`arrow_back U+E092`.

**Step 2: Write the failing tests**

`theme.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_match_the_flutter_values() {
        assert_eq!(BACKGROUND, Color::BLACK);
        assert_eq!(TEXT, Color::WHITE);
        assert_eq!(MUTED, Color::from_rgb8(0xCC, 0xCC, 0xCC));
        assert_eq!(DISABLED, Color::from_rgb8(0x33, 0x33, 0x33));
        assert!((BORDER.a - 0.24).abs() < f32::EPSILON);
        assert!((CARD.a - 0.10).abs() < f32::EPSILON);
        assert!((WHITE_54.a - 0.54).abs() < f32::EPSILON);
    }

    #[test]
    fn radii_and_metrics_are_fixed() {
        assert_eq!(RADIUS, 12.0);
        assert_eq!(BUTTON_BORDER, 2.0);
        assert_eq!(DASHBOARD_HEIGHT, 480.0);
        assert_eq!(ROW_HEIGHT, 90.0);
        assert_eq!(SPEED_SIZE, 100.0);
    }
}
```

`fonts.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_font_is_registered_last() {
        let fonts = all();
        assert_eq!(fonts.last().map(|font| font.as_ref()), Some(ICON_FONT_BYTES));
    }

    #[test]
    fn the_bundled_font_is_an_open_type_file() {
        assert_eq!(&ICON_FONT_BYTES[..4], b"OTTO");
        assert_eq!(ICON_FONT.family, iced::font::Family::Name("Material Icons"));
    }
}
```

`icons.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codepoints_match_the_bundled_font() {
        assert_eq!(GPS_FIXED, '\u{e2dc}');
        assert_eq!(GPS_NOT_FIXED, '\u{e2dd}');
        assert_eq!(BLUETOOTH, '\u{e0e4}');
        assert_eq!(BLUETOOTH_CONNECTED, '\u{e0e6}');
        assert_eq!(BLUETOOTH_DISABLED, '\u{e0e7}');
        assert_eq!(BLUETOOTH_SEARCHING, '\u{e0e9}');
        assert_eq!(SEARCH, '\u{e567}');
        assert_eq!(STOP, '\u{e606}');
        assert_eq!(ARROW_BACK_IOS, '\u{e093}');
        assert_eq!(CHEVRON_RIGHT, '\u{e15f}');
    }
}
```

**Step 3: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp theme:: fonts:: icons::
```

**Step 4: Implement**

`app/rust/Cargo.toml`: add `"zipp"` to `members`.

`app/rust/zipp/Cargo.toml`:

```toml
[package]
name = "zipp"
version.workspace = true
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "zipp"
path = "src/main.rs"

[dependencies]
zipp_core = { path = "../zipp_core" }
iced = { git = "https://github.com/Santuzius/iced", tag = "iced_android-v0.1.1", default-features = false, features = ["wgpu", "tokio", "image", "wayland", "advanced", "web-colors", "canvas"] }
iced_android = { git = "https://github.com/Santuzius/iced_android", tag = "v0.1.1" }
tokio = { version = "1", features = ["sync", "rt-multi-thread", "time"] }
futures = { version = "0.3", features = ["executor"] }
log = "0.4"
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
reqwest = { version = "0.12", default-features = false }

[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.15"
jni = "0.21"

[target.'cfg(not(target_os = "android"))'.dependencies]
btleplug = "0.13.5"
```

`theme.rs` tokens:

```rust
use iced::Color;

pub const BACKGROUND: Color = Color::BLACK;
pub const TEXT: Color = Color::WHITE;
pub const MUTED: Color = Color::from_rgb8(0xCC, 0xCC, 0xCC);
pub const DISABLED: Color = Color::from_rgb8(0x33, 0x33, 0x33);
pub const ACTIVE_FILL: Color = DISABLED;
pub const BORDER: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.24 };
pub const CARD: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 };
pub const WHITE_54: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.54 };

pub const RADIUS: f32 = 12.0;
pub const BUTTON_BORDER: f32 = 2.0;
pub const DASHBOARD_HEIGHT: f32 = 480.0;
pub const ROW_HEIGHT: f32 = 90.0;

pub const SPEED_SIZE: f32 = 100.0;
pub const STAT_SIZE: f32 = 35.0;
pub const AMPM_SIZE: f32 = 25.0;
pub const BOTTOM_STAT_SIZE: f32 = 26.0;
pub const UNIT_SIZE: f32 = 24.0;
pub const BUTTON_SIZE: f32 = 20.0;
pub const TITLE_SIZE: f32 = 28.0;
pub const LIST_TITLE_SIZE: f32 = 18.0;
pub const LIST_SUBTITLE_SIZE: f32 = 14.0;
pub const LOG_SIZE: f32 = 16.0;
pub const GPS_SIZE: f32 = 12.0;
pub const STATUS_SIZE: f32 = 20.0;
pub const DEVICE_ID_SIZE: f32 = 12.0;
pub const ICON_SIZE: f32 = 32.0;
pub const SCAN_ICON_SIZE: f32 = 28.0;
pub const STOP_ICON_SIZE: f32 = 32.0;
pub const LOG_INPUT_HEIGHT: f32 = 60.0;
pub const LOG_MAX_LINE_LENGTH: f32 = 0.0; // unused; remove if clippy warns
```

Remove the unused constant; do not keep filler values.

`fonts.rs`:

```rust
use std::borrow::Cow;

use iced::Font;

pub const DEFAULT_FONT: Font = iced_android::DEFAULT_FONT;
pub const ICON_FONT: Font = Font::with_name("Material Icons");
pub const ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/MaterialIcons-Regular.otf");

pub fn all() -> Vec<Cow<'static, [u8]>> {
    let mut fonts = iced_android::fonts();
    if let Some(cjk) = desktop_cjk_font() {
        fonts.push(Cow::Owned(cjk));
    }
    fonts.push(Cow::Borrowed(ICON_FONT_BYTES));
    fonts
}

#[cfg(target_os = "linux")]
fn desktop_cjk_font() -> Option<Vec<u8>> {
    const PATHS: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
    ];
    PATHS.iter().find_map(|path| std::fs::read(path).ok())
}

#[cfg(not(target_os = "linux"))]
fn desktop_cjk_font() -> Option<Vec<u8>> {
    None
}
```

`icons.rs`:

```rust
pub use crate::fonts::ICON_FONT as FONT;

pub const GPS_FIXED: char = '\u{e2dc}';
pub const GPS_NOT_FIXED: char = '\u{e2dd}';
pub const BLUETOOTH: char = '\u{e0e4}';
pub const BLUETOOTH_CONNECTED: char = '\u{e0e6}';
pub const BLUETOOTH_DISABLED: char = '\u{e0e7}';
pub const BLUETOOTH_SEARCHING: char = '\u{e0e9}';
pub const SEARCH: char = '\u{e567}';
pub const STOP: char = '\u{e606}';
pub const ARROW_BACK_IOS: char = '\u{e093}';
pub const CHEVRON_RIGHT: char = '\u{e15f}';

pub fn glyph(character: char) -> String {
    character.to_string()
}
```

`lib.rs` starts with:

```rust
//! Zipp: an iced dashboard for the BScooter.

pub mod fonts;
pub mod icons;
pub mod theme;
```

`main.rs`:

```rust
fn main() -> iced::Result {
    zipp::run()
}
```

Add a temporary `pub fn run() -> iced::Result { todo!() }` only if it is
needed to compile this task; Task 7 replaces it. Prefer creating Task 7's
skeleton in this task if `todo!()` bothers you. Either way `cargo test -p zipp`
must link the binary, so `main.rs` must call an existing `run`.

**Step 5: Re-run, build once, commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp theme:: fonts:: icons::
cargo check -p zipp
cd ../..
git add app/rust/Cargo.toml app/rust/Cargo.lock app/rust/zipp
git commit -m "feat(ui): add iced crate, theme, fonts and icons"
```

The first `cargo check` downloads the pinned git dependencies and can take
several minutes.

---

## Task 7: Add the app skeleton, messages, window and run loop

**Files**

- Create: `app/rust/zipp/src/app.rs`
- Create: `app/rust/zipp/src/message.rs`
- Create: `app/rust/zipp/src/update.rs`
- Create: `app/rust/zipp/src/view.rs`
- Create: `app/rust/zipp/src/platform/mod.rs`
- Modify: `app/rust/zipp/src/lib.rs`

**Step 1: Write the failing window test**

In `app.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_one_resizable_window_at_the_flutter_size() {
        let settings = window_settings();
        assert_eq!(settings.size, Size::new(420.0, 820.0));
        assert_eq!(settings.min_size, Some(Size::new(360.0, 640.0)));
        assert!(settings.resizable);
    }
}
```

**Step 2: Run and watch it fail**

```sh
cd app/rust
cargo test -p zipp window_settings
```

**Step 3: Implement**

`platform/mod.rs` (temporary no-op transport; Task 13 replaces the desktop arm):

```rust
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
    TRANSPORT.get_or_init(|| Arc::new(NoopTransport::new())).clone()
}

pub fn lang() -> Lang {
    zipp_core::i18n::detect()
}
```

`app.rs`:

```rust
use std::sync::Arc;

use chrono::NaiveTime;
use iced::window;
use iced::{Size, Task};
use zipp_core::i18n::Lang;
use zipp_core::session::ScooterSession;
use zipp_core::transport::Transport;

use crate::message::Message;
use crate::platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Log,
}

pub struct App {
    pub session: ScooterSession,
    pub transport: Arc<dyn Transport>,
    pub page: Page,
    pub uart_input: String,
    pub insets: iced_android::Insets,
    pub scale: f32,
    pub now: NaiveTime,
}

pub fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(420.0, 820.0),
        min_size: Some(Size::new(360.0, 640.0)),
        resizable: true,
        ..Default::default()
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        platform::ensure_transport();
        let (_id, open) = window::open(window_settings());
        iced_android::set_system_bars_dark(true);
        let lang = platform::lang();
        let app = Self::with_transport(platform::transport(), lang);
        (app, open.map(|_| Message::Noop))
    }

    pub fn with_transport(transport: Arc<dyn Transport>, lang: Lang) -> Self {
        Self {
            session: ScooterSession::new(lang),
            transport,
            page: Page::Dashboard,
            uart_input: String::new(),
            insets: iced_android::Insets::default(),
            scale: 1.0,
            now: zipp_core::log::now(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(transport: Arc<dyn Transport>) -> Self {
        Self::with_transport(transport, Lang::En)
    }
}
```

`message.rs` starts minimal and grows in later tasks:

```rust
use iced::window;
use zipp_core::transport::TransportEvent;

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    Tick,
    Transport(TransportEvent),
    WindowOpened(window::Id),
}
```

`update.rs`:

```rust
use iced::Task;

use crate::app::App;
use crate::message::Message;

impl App {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop | Message::WindowOpened(_) => Task::none(),
            Message::Tick => {
                self.now = zipp_core::log::now();
                Task::none()
            }
            Message::Transport(_event) => Task::none(),
        }
    }
}
```

`view.rs` placeholder:

```rust
use iced::widget::{container, text};
use iced::{Element, Length};

use crate::app::App;
use crate::message::Message;
use crate::theme;

pub fn view(app: &App, _window: iced::window::Id) -> Element<'_, Message> {
    container(text("Zipp").size(24).color(theme::TEXT))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BACKGROUND.into()),
            ..Default::default()
        })
        .into()
}
```

`lib.rs`:

```rust
pub mod app;
pub mod fonts;
pub mod icons;
pub mod message;
mod platform;
pub mod theme;
pub mod update;
pub mod view;

pub use app::App;
pub use message::Message;

pub fn run() -> iced::Result {
    let mut daemon = iced::daemon(App::new, App::update, App::view)
        .title(|_, _| "Zipp".into())
        .theme(|_, _| iced::Theme::Dark)
        .style(|_, _| iced::theme::Style {
            background_color: theme::BACKGROUND,
            text_color: theme::TEXT,
        })
        .subscription(App::subscription)
        .scale_factor(|app, _| app.scale)
        .default_font(fonts::DEFAULT_FONT);

    for font in fonts::all() {
        daemon = daemon.font(font);
    }

    daemon.run()
}
```

Add `App::subscription` in `update.rs` now, returning `Subscription::batch([iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)])`. Task 11 adds transport and Android subscriptions.

**Step 4: Run and smoke-test**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cargo run -p zipp
```

A 420x820 black window with white "Zipp" appears. Close it with the window
manager.

**Step 5: Commit**

```sh
cd ../..
git add app/rust/zipp
git commit -m "feat(ui): add app skeleton and daemon"
```

---

## Task 8: Add the shared component kit

**Files**

- Create: `app/rust/zipp/src/components/mod.rs`
- Create: `app/rust/zipp/src/components/chrome.rs`
- Create: `app/rust/zipp/src/components/stat.rs`
- Create: `app/rust/zipp/src/components/button.rs`
- Modify: `app/rust/zipp/src/lib.rs` (`pub mod components;`)

**Step 1: Write the failing style tests**

In `button.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_fill_is_present_only_when_on() {
        assert_eq!(fill(Some(true)), Some(theme::ACTIVE_FILL));
        assert_eq!(fill(Some(false)), None);
        assert_eq!(fill(None), None);
    }

    #[test]
    fn an_action_button_never_shows_on_off() {
        assert!(!shows_on_off(None));
        assert!(shows_on_off(Some(false)));
        assert!(shows_on_off(Some(true)));
    }
}
```

**Step 2: Run and watch it fail**

```sh
cd app/rust
cargo test -p zipp fill
```

**Step 3: Implement**

`chrome.rs`:

```rust
use iced::widget::{container, space, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::theme;

pub fn divider() -> Element<'static, Message> {
    container(space::horizontal())
        .width(Length::Fill)
        .height(theme::BUTTON_BORDER)
        .style(|_| container::Style {
            background: Some(theme::BORDER.into()),
            ..Default::default()
        })
        .into()
}

pub fn icon(character: char, size: f32, color: iced::Color) -> Element<'static, Message> {
    text(character.to_string())
        .font(crate::icons::FONT)
        .size(size)
        .color(color)
        .into()
}
```

`stat.rs`:

```rust
use iced::widget::text;
use iced::Element;

use crate::message::Message;
use crate::theme;

pub fn stat<'a>(value: impl Into<String>, size: f32) -> Element<'a, Message> {
    text(value.into()).size(size).color(theme::MUTED).into()
}

pub fn unit<'a>(value: impl Into<String>, size: f32) -> Element<'a, Message> {
    text(value.into()).size(size).color(theme::MUTED).into()
}
```

`button.rs`:

```rust
use iced::widget::{button, column, container, text};
use iced::{Border, Element, Length, Padding};

use crate::message::Message;
use crate::theme;

pub fn fill(on: Option<bool>) -> Option<iced::Color> {
    on.filter(|value| *value).map(|_| theme::ACTIVE_FILL)
}

pub fn shows_on_off(on: Option<bool>) -> bool {
    on.is_some()
}

pub fn dashboard(
    label: impl Into<String>,
    on: Option<bool>,
    on_off: [&str; 2],
    message: Message,
) -> Element<'static, Message> {
    let mut body = column![text(label.into())
        .size(theme::BUTTON_SIZE)
        .color(theme::TEXT)
        .align_x(iced::alignment::Horizontal::Center)]
    .align_x(iced::Alignment::Center)
    .spacing(0);
    if let Some(value) = on {
        body = body.push(
            text(if value { on_off[0] } else { on_off[1] })
                .size(theme::BUTTON_SIZE)
                .color(theme::TEXT),
        );
    }
    let background = fill(on);
    button(container(body).center_x(Length::Fill).center_y(Length::Fill))
        .on_press(message)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| button::Style {
            background: background.map(Into::into),
            text_color: theme::TEXT,
            border: Border {
                color: theme::BORDER,
                width: theme::BUTTON_BORDER,
                radius: theme::RADIUS.into(),
            },
            ..button::Style::default()
        })
        .into()
}
```

Every dashboard button sits in `container(button).padding(4)`; the parent row
gives the height.

**Step 4: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cd ../..
git add app/rust/zipp
git commit -m "feat(ui): add shared component kit"
```

---

## Task 9: Compose the dashboard with Flutter parity

**Files**

- Modify: `app/rust/zipp/src/view.rs`
- Modify: `app/rust/zipp/src/message.rs`

**Step 1: Write the failing presentation tests**

In `view.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zipp_core::gps::GpsStatus;
    use zipp_core::protocol::ScooterState;

    #[test]
    fn twelve_hour_clock_matches_flutter() {
        assert_eq!(clock_12h(0, 5), ("12:05".into(), "am"));
        assert_eq!(clock_12h(11, 59), ("11:59".into(), "am"));
        assert_eq!(clock_12h(12, 0), ("12:00".into(), "pm"));
        assert_eq!(clock_12h(13, 7), ("1:07".into(), "pm"));
        assert_eq!(clock_12h(23, 59), ("11:59".into(), "pm"));
    }

    #[test]
    fn empty_values_use_the_flutter_defaults() {
        assert_eq!(value(Some(""), "00"), "00");
        assert_eq!(value(None, "00"), "00");
        assert_eq!(value(Some("42"), "00"), "42");
    }

    #[test]
    fn indicators_default_active_until_state_arrives() {
        assert!(indicator(None, Indicator::Brake));
        assert!(indicator(None, Indicator::Cruise));
        assert!(indicator(None, Indicator::Park));
        let state = ScooterState { brake_active: false, park_brake_active: false, ..Default::default() };
        assert!(!indicator(Some(&state), Indicator::Brake));
        assert!(!indicator(Some(&state), Indicator::Cruise));
        assert!(!indicator(Some(&state), Indicator::Park));
    }

    #[test]
    fn toggles_default_off() {
        assert!(!toggle(None, Toggle::Power));
        let state = ScooterState { power_on: true, ..Default::default() };
        assert!(toggle(Some(&state), Toggle::Power));
    }

    #[test]
    fn gps_speed_replaces_the_state_speed_only_when_active() {
        let state = ScooterState { speed: "12".into(), ..Default::default() };
        assert_eq!(speed_text(Some(&state), GpsStatus::Active, Some(7.4)), "07");
        assert_eq!(speed_text(Some(&state), GpsStatus::Searching, Some(7.4)), "12");
        assert_eq!(speed_text(None, GpsStatus::Disabled, None), "00");
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp clock_12h
```

**Step 3: Implement the helpers**

```rust
use zipp_core::gps::GpsStatus;
use zipp_core::protocol::ScooterState;

pub fn value(value: Option<&str>, fallback: &str) -> String {
    match value {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => fallback.to_string(),
    }
}

pub fn clock_12h(hour: u32, minute: u32) -> (String, &'static str) {
    let hour = if hour > 12 { hour - 12 } else if hour == 0 { 12 } else { hour };
    (format!("{hour}:{minute:02}"), if hour >= 12 { "pm" } else { "am" })
}
```

Careful: Flutter checks `now.hour >= 12` for `pm` before converting. The
implementation above must use the original hour for `am`/`pm`:

```rust
pub fn clock_12h(hour: u32, minute: u32) -> (String, &'static str) {
    let pm = hour >= 12;
    let hour = if hour > 12 { hour - 12 } else if hour == 0 { 12 } else { hour };
    (format!("{hour}:{minute:02}"), if pm { "pm" } else { "am" })
}
```

```rust
pub enum Indicator { Brake, Cruise, Park }
pub enum Toggle { Power, Alarm, Lights, Sport }

pub fn indicator(state: Option<&ScooterState>, which: Indicator) -> bool {
    match (state, which) {
        (None, _) => true,
        (Some(s), Indicator::Brake) => s.brake_active,
        (Some(s), Indicator::Cruise) => s.cruise_level > 0,
        (Some(s), Indicator::Park) => s.park_brake_active,
    }
}

pub fn toggle(state: Option<&ScooterState>, which: Toggle) -> bool {
    let Some(s) = state else { return false };
    match which {
        Toggle::Power => s.power_on,
        Toggle::Alarm => s.alarm_on,
        Toggle::Lights => s.lights_on,
        Toggle::Sport => s.sport_on,
    }
}

pub fn speed_text(
    state: Option<&ScooterState>,
    status: GpsStatus,
    gps_speed_kmh: Option<f64>,
) -> String {
    if status == GpsStatus::Active {
        if let Some(speed) = gps_speed_kmh {
            return format!("{:02}", speed.round() as i64);
        }
    }
    value(state.map(|s| s.speed.as_str()), "00")
}
```

**Step 4: Build the dashboard element**

Replace the placeholder `view` with a root that switches pages and pads by
insets. `Message` gains:

```rust
BeginScan,
StopScan,
ScanToggle,
Connect(zipp_core::transport::ScanDevice),
Disconnect,
Send(zipp_core::protocol::ScooterCommand),
Entry,
OpenLog,
Back,
ClearLogs,
UartChanged(String),
UartSubmit,
Gps(zipp_core::gps::GpsUpdate),
Insets(iced_android::Insets),
SystemScale(f32),
```

`dashboard(app)` builds this exact tree:

1. `row1` height 90: four `components::button::dashboard` children, each wrapped
   in `container(...).padding(4).width(Length::Fill)`.
   Power/Alarm/Lights call `Message::Send(ScooterCommand::Toggle*)` with
   `Some(toggle(state, ...))`; Entry calls `Message::Entry` with `None`.
2. `row2`: `container(row![left, clock, right].padding(Padding { top: 4, right: 10, bottom: 4, left: 10 }))`.
   - left `stat::stat(format!("{}°C", value(state.temperature, "00")), STAT_SIZE)`
     aligned left in a fill container.
   - clock `row![stat::stat(time, STAT_SIZE), stat::stat(format!(" {ampm}"), AMPM_SIZE)].align_y(Bottom)`
   - right `stat::stat(format!("{}%", value(state.battery_level, "0")), STAT_SIZE)`
     aligned right.
3. `chrome::divider()`.
4. `row3` height `Fill`, horizontal padding 10:
   - left `container(column![...].height(Fill)).width(100)`: each of Brake,
     Cruise, Park in a `container(...).height(Fill).center_y(Length::Fill)`,
     `stat::stat(label, BOTTOM_STAT_SIZE)` coloured `TEXT` when active and
     `DISABLED` when not. Use `.color(...)`; `stat` is only a convenience for
     the muted colour, so build these with `text` directly.
   - centre `column![gps_row, container(speed_row).height(Fill).center_y(Length::Fill), throttle].align_x(Center)`.
     Flutter's centre `Column` centres its children horizontally, and iced
     columns default to start alignment, so the align_x is required for parity.
     `gps_row` is a `row![icon, text("GPS").size(GPS_SIZE).color(MUTED)]` with
     4 px gap, shown only for `Active`/`Searching`; icon is `GPS_FIXED` for
     active and `GPS_NOT_FIXED` for searching, size 16, wrapped in top padding 12.
     `speed_row` is `row![stat::stat(speed_text(...), SPEED_SIZE), stat::unit(format!(" {}", strings.kmh), UNIT_SIZE)].align_y(Bottom)`.
     `throttle` is `stat::unit(format!("{} {}", value(state.throttle_level, "0000"), strings.mv), UNIT_SIZE)`
     with bottom padding 8.
   - right `container(column![...].height(Fill)).width(100)`: five bars for
     `level in (1..=5).rev()`, each `container(bar).padding([4, 0]).height(Fill)`;
     wrap the whole column in vertical padding 10 to match Flutter.
     `bar` is `container(space::horizontal()).width(20.0 + level as f32 * 15.0).height(Fill)`
     with `background = MUTED` when `state.map_or(true, |s| s.cruise_level >= level)`
     otherwise `DISABLED`, and `Border { radius: 100.0.into(), ..default }`.
     Align each bar right.
5. `chrome::divider()`.
6. `row4`: odometer left (`{} {}` with `strings.km`), battery power centre
   (`{} {}` with `strings.watts`), clock right (`HH:MM` from the protocol, not
   the wall clock), all `BOTTOM_STAT_SIZE`, padding `[8, 10]`.
7. `row5` height 90: Sport (toggle), Cruise Down, Cruise Up, Horn. Cruise
   Down/Up and Horn are action buttons (`None`).

Wrap the whole dashboard in `container(column![...]).height(Length::Fixed(DASHBOARD_HEIGHT))`.
The page then puts the scanner below it in a `column![dashboard, scanner].height(Fill)`.

**Step 5: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cargo run -p zipp
cd ../..
git add app/rust/zipp
git commit -m "feat(ui): compose the dashboard"
```

The window now shows the static dashboard with default values (clock running
only after Task 11 wires the tick).

---

## Task 10: Add the connectivity row, scanner list and log page

**Files**

- Create: `app/rust/zipp/src/components/scanner.rs`
- Create: `app/rust/zipp/src/components/log.rs`
- Modify: `app/rust/zipp/src/components/mod.rs`
- Modify: `app/rust/zipp/src/view.rs`

**Step 1: Write the failing status tests**

In `scanner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zipp_core::session::ScooterSession;
    use zipp_core::i18n::Lang;

    #[test]
    fn status_prefers_connected_then_connecting_then_scanning() {
        let mut s = ScooterSession::new(Lang::En);
        assert_eq!(status(&s), Status::Disconnected);
        s.scanning = true;
        assert_eq!(status(&s), Status::Scanning);
        s.connecting = true;
        assert_eq!(status(&s), Status::Connecting);
        s.connected = Some(zipp_core::session::ConnectedDevice {
            id: zipp_core::transport::DeviceId("AA".into()),
            name: "BScooter".into(),
        });
        assert_eq!(status(&s), Status::Connected);
    }

    #[test]
    fn the_scan_button_stops_an_active_scan() {
        assert_eq!(scan_message(true), crate::message::Message::StopScan);
        assert_eq!(scan_message(false), crate::message::Message::BeginScan);
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp status_prefers
```

**Step 3: Implement the scanner**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Connected,
    Connecting,
    Scanning,
    Disconnected,
}

pub fn status(session: &ScooterSession) -> Status { ... }
pub fn scan_message(scanning: bool) -> Message {
    if scanning { Message::StopScan } else { Message::BeginScan }
}
```

Build the elements:

- `status_row(app)`:
  - icon 32: `BLUETOOTH_CONNECTED` when connected,
    `BLUETOOTH_SEARCHING` when connecting/scanning, `BLUETOOTH_DISABLED`
    otherwise, colour `MUTED`.
  - text `STATUS_SIZE`, bold
    (`Font { weight: iced::font::Weight::Bold, ..fonts::DEFAULT_FONT }`):
    connected name or `strings.unknown`, `strings.connecting`,
    `strings.scanning`, `strings.disconnected`.
  - device id `DEVICE_ID_SIZE`, `WHITE_54`, shown only when connected.
  - scan button: `STOP` at 32 when scanning, `SEARCH` at 28 otherwise; press
    `scan_message`.
  - padding `[16, 24]` (vertical, horizontal).
  - Wrap the left icon+text block in a transparent `button` with
    `on_press(Message::OpenLog)` and `width(Fill)`, then place the scan button
    beside it in a row. This avoids a whole-row `mouse_area` swallowing the scan
    press.
- `chrome::divider()`.
- `scanner_list(app)`:
  - `scrollable(column![...]).height(Fill)` with horizontal padding 8.
  - Connected card: `container(row![icon, name, disconnect_button])
    .padding(12).style(background CARD, radius RADIUS)`.
    Name is `LIST_TITLE_SIZE`; disconnect button text `LIST_SUBTITLE_SIZE`
    muted and `on_press(Message::Disconnect)`.
  - Device rows: for each device not equal to the connected id, a transparent
    `button(row![BLUETOOTH icon, column![name 18, id 14]])
    .on_press(Message::Connect(device.clone())).padding(12).width(Fill)`.
    Empty names become `strings.unknown`.
  - Empty state when `devices.is_empty() && !scanning && connected.is_none()`:
    centred `"No devices found"`, `GREY` (0x9E9E9E, Flutter `Colors.grey`),
    `LOG_SIZE`, padding 16.
- `page(app)`:
  - `column![status_row, divider, scanner_list]` in a fill container with black
    background.

**Step 4: Build the log page**

`log.rs`:

```rust
pub const LOG_SCROLL: &str = "zipp-log-scroll";

pub fn page(app: &App) -> Element<'_, Message> { ... }
pub fn terminal(app: &App) -> Element<'_, Message> { ... }
pub fn input_style() -> impl Fn(&iced::Theme, iced::widget::text_input::Status)
    -> iced::widget::text_input::Style + 'static { ... }
```

Layout:

- Header: `row![back_button, gap 8, text(title).size(TITLE_SIZE).bold()]
  .padding([12,16])`; back icon `ARROW_BACK_IOS` size 24 on a transparent
  button, `on_press(Message::Back)`.
- `chrome::divider()`.
- Terminal:
  - `scrollable(column![for line in logs { text(line).size(LOG_SIZE).color(MUTED).line_height(1.2) }])
    .id(LOG_SCROLL).padding(10).height(Fill)` inside `stack![scroll, clear]`.
  - Clear button top-right: `container(clear_button).padding(10).align_x(Right).align_y(Top)`;
    button background `ACTIVE_FILL`, radius 4, text `strings.clear` size 16
    white, padding `[8,12]`, `on_press(Message::ClearLogs)`.
  - Input row height 60, padding horizontal 10, top border white-24 1 px:
    `text_input("...", &app.uart_input).on_input(Message::UartChanged)
    .on_submit(Message::UartSubmit).size(18).padding(0).style(input_style())`.
    The style returns `background: transparent`, `border: Border::default()`,
    `icon: MUTED`, `placeholder: BORDER` (white 0.24, Flutter `Colors.white24`;
    the input's text size is already 18), `value: TEXT`, `selection` white 35%.
  - Send button: `CHEVRON_RIGHT` 30 white on `ACTIVE_FILL`, `on_press(Message::UartSubmit)`.

**Step 5: Switch pages and decide the transition**

`view::view`:

```rust
let body = match app.page {
    Page::Dashboard => dashboard_page(app),
    Page::Log => components::log::page(app),
};
container(body)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(app.insets)
    .style(|_| container::Style {
        background: Some(theme::BACKGROUND.into()),
        ..Default::default()
    })
    .into()
```

**Page transition decision:** implement an instant swap for v1. A faithful
300 ms slide needs two live pages, clipping and hit-testing, which `float` does
not provide cleanly. Record this single deviation in
`app/docs/technical-notes.md` (Task 20) as "dashboard/log switch is instant
instead of a 300 ms slide". Do not spend time on a custom animation widget
before the Android build works.

**Step 6: Re-run and commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cargo run -p zipp
cd ../..
git add app/rust/zipp
git commit -m "feat(ui): add scanner and log page"
```

---

## Task 11: Wire the Elm update loop and subscriptions

**Files**

- Modify: `app/rust/zipp/src/message.rs`
- Modify: `app/rust/zipp/src/update.rs`
- Modify: `app/rust/zipp/src/platform/mod.rs`
- Modify: `app/rust/zipp/src/view.rs` (only if the page switch needs it)

**Step 1: Add a fake transport and write the failing tests**

In `platform/mod.rs`:

```rust
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
            Self { events, wifi, sent: Mutex::new(Vec::new()) }
        }

        pub fn emit(&self, event: TransportEvent) {
            let _ = self.events.send(event);
        }

        pub fn sent(&self) -> Vec<TransportCommand> {
            self.sent.lock().unwrap().clone()
        }

        pub fn set_wifi(&self, value: bool) {
            let _ = self.wifi.send(value);
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
```

In `update.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::app::{App, Page};
    use crate::message::Message;
    use crate::platform::fake::FakeTransport;
    use zipp_core::protocol::ScooterCommand;
    use zipp_core::transport::{DeviceId, ScanDevice, TransportCommand, TransportEvent};

    fn app() -> (App, Arc<FakeTransport>) {
        let transport = Arc::new(FakeTransport::new());
        (App::for_test(transport.clone()), transport)
    }

    #[test]
    fn scan_toggle_starts_then_stops() {
        let (mut app, transport) = app();
        app.update(Message::ScanToggle);
        assert_eq!(
            transport.sent(),
            vec![TransportCommand::StartScan { target: "BScooter".into() }]
        );
        app.session.scanning = true;
        app.update(Message::ScanToggle);
        assert_eq!(
            transport.sent().last(),
            Some(&TransportCommand::StopScan)
        );
    }

    #[test]
    fn reconnect_tick_only_runs_when_idle() {
        let (mut app, transport) = app();
        app.update(Message::ReconnectTick);
        assert_eq!(
            transport.sent(),
            vec![TransportCommand::Reconnect { target: "BScooter".into() }]
        );
        app.session.scanning = true;
        app.update(Message::ReconnectTick);
        assert_eq!(transport.sent().len(), 1);
    }

    #[test]
    fn scan_results_auto_connect_and_stop_the_scan() {
        let (mut app, transport) = app();
        app.session.scanning = true;
        app.update(Message::Transport(TransportEvent::Devices(vec![ScanDevice {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
            rssi: Some(-40),
        }])));
        assert_eq!(
            transport.sent(),
            vec![
                TransportCommand::StopScan,
                TransportCommand::Connect { id: DeviceId("AA".into()) }
            ]
        );
    }

    #[test]
    fn a_connected_scooter_accepts_writes() {
        let (mut app, transport) = app();
        app.update(Message::Transport(TransportEvent::Connected {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
        }));
        app.update(Message::Send(ScooterCommand::TogglePower));
        assert!(matches!(
            transport.sent().last(),
            Some(TransportCommand::Write { bytes, .. }) if bytes == &vec![183]
        ));
    }

    #[test]
    fn back_returns_to_the_dashboard() {
        let (mut app, _) = app();
        app.update(Message::OpenLog);
        assert_eq!(app.page, Page::Log);
        app.update(Message::Back);
        assert_eq!(app.page, Page::Dashboard);
        app.update(Message::Back);
        assert_eq!(app.page, Page::Dashboard);
    }

    #[test]
    fn uart_submit_sends_the_text_and_clears_the_field() {
        let (mut app, transport) = app();
        app.update(Message::Transport(TransportEvent::Connected {
            id: DeviceId("AA".into()),
            name: "BScooter".into(),
        }));
        app.update(Message::UartChanged("hello".into()));
        app.update(Message::UartSubmit);
        assert!(app.uart_input.is_empty());
        assert!(matches!(
            transport.sent().last(),
            Some(TransportCommand::Write { bytes, .. }) if bytes == b"hello"
        ));
    }

    #[test]
    fn empty_uart_input_does_nothing() {
        let (mut app, transport) = app();
        app.update(Message::UartSubmit);
        assert!(transport.sent().is_empty());
    }

    #[test]
    fn entry_waits_for_wifi_and_sends_once() {
        let (mut app, transport) = app();
        // The wifi-wait path only exists on mobile; force it so the test is
        // deterministic on a desktop runner.
        app.mobile = true;
        transport.set_wifi(false);
        app.update(Message::Entry);
        assert!(transport.sent().is_empty());
        // The pending flag is set; a later Wifi(true) starts the HTTP task.
        assert!(app.session.entry_pending);
        app.update(Message::Transport(TransportEvent::Wifi(true)));
        assert!(!app.session.entry_pending);
    }

    #[test]
    fn clear_logs_empties_the_buffer() {
        let (mut app, _) = app();
        app.session.log("hello", chrono::NaiveTime::from_hms_opt(1, 2, 0).unwrap());
        app.update(Message::ClearLogs);
        assert!(app.session.logs.is_empty());
    }

    #[test]
    fn insets_and_scale_are_stored() {
        let (mut app, _) = app();
        app.update(Message::Insets(iced_android::Insets { top: 24.0, right: 1.0, bottom: 2.0, left: 3.0 }));
        app.update(Message::SystemScale(1.25));
        assert_eq!(app.insets.top, 24.0);
        assert!((app.scale - 1.25).abs() < f32::EPSILON);
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp scan_toggle_starts
```

**Step 3: Implement the update loop**

`message.rs` full enum:

```rust
use iced::window;
use zipp_core::gps::GpsUpdate;
use zipp_core::protocol::ScooterCommand;
use zipp_core::transport::{ScanDevice, TransportEvent};

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    Tick,
    ReconnectTick,
    Transport(TransportEvent),
    BeginScan,
    StopScan,
    ScanToggle,
    Connect(ScanDevice),
    Disconnect,
    Send(ScooterCommand),
    Entry,
    EntryFinished(Result<u16, String>),
    OpenLog,
    Back,
    ClearLogs,
    UartChanged(String),
    UartSubmit,
    Gps(GpsUpdate),
    Insets(iced_android::Insets),
    SystemScale(f32),
    WindowOpened(window::Id),
}
```

`update.rs` (replace the Task 7 match):

```rust
use std::time::Duration;

use iced::{Subscription, Task};
use zipp_core::protocol::ScooterCommand;
use zipp_core::transport::{TransportCommand, TransportEvent};

use crate::app::{App, Page};
use crate::message::Message;

fn scroll_logs() -> Task<Message> {
    iced::widget::operation::snap_to_end(crate::components::log::LOG_SCROLL)
}

impl App {
    fn send_all(&self, commands: Vec<TransportCommand>) {
        for command in commands {
            self.transport.send(command);
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop | Message::WindowOpened(_) => Task::none(),
            Message::Tick => {
                self.now = zipp_core::log::now();
                Task::none()
            }
            Message::ReconnectTick => {
                let commands = self.session.tick(self.now);
                self.send_all(commands);
                Task::none()
            }
            Message::Transport(event) => {
                let commands = self.session.apply(event.clone(), self.now);
                self.send_all(commands);
                if matches!(event, TransportEvent::Wifi(true)) && self.session.entry_pending {
                    self.session.finish_entry();
                    return Task::perform(crate::platform::send_entry(), Message::EntryFinished);
                }
                if matches!(event, TransportEvent::Wifi(_)) {
                    Task::none()
                } else {
                    scroll_logs()
                }
            }
            Message::BeginScan => {
                let commands = self.session.begin_scan();
                self.send_all(commands);
                Task::none()
            }
            Message::StopScan => {
                let commands = self.session.stop_scan();
                self.send_all(commands);
                Task::none()
            }
            Message::ScanToggle => {
                let commands = if self.session.scanning {
                    self.session.stop_scan()
                } else {
                    self.session.begin_scan()
                };
                self.send_all(commands);
                Task::none()
            }
            Message::Connect(device) => {
                let commands = self.session.connect(device, self.now);
                self.send_all(commands);
                scroll_logs()
            }
            Message::Disconnect => {
                let commands = self.session.disconnect();
                self.send_all(commands);
                Task::none()
            }
            Message::Send(command) => {
                let commands = self.session.send_command(command, self.now);
                self.send_all(commands);
                scroll_logs()
            }
            Message::Entry => {
                let mobile = self.mobile;
                let wifi = *self.transport.wifi().borrow();
                match self.session.request_entry(mobile, wifi) {
                    zipp_core::entry::EntryDecision::SendNow => {
                        Task::perform(crate::platform::send_entry(), Message::EntryFinished)
                    }
                    zipp_core::entry::EntryDecision::WaitForWifi
                    | zipp_core::entry::EntryDecision::Ignore => Task::none(),
                }
            }
            Message::EntryFinished(result) => {
                match result {
                    Ok(status) => log::info!("Entry request sent. Status: {status}"),
                    Err(error) => log::warn!("{error}"),
                }
                self.session.finish_entry();
                Task::none()
            }
            Message::OpenLog => {
                self.page = Page::Log;
                scroll_logs()
            }
            Message::Back => {
                self.page = Page::Dashboard;
                Task::none()
            }
            Message::ClearLogs => {
                self.session.logs.clear();
                Task::none()
            }
            Message::UartChanged(value) => {
                self.uart_input = value;
                Task::none()
            }
            Message::UartSubmit => {
                let value = std::mem::take(&mut self.uart_input);
                if value.is_empty() {
                    Task::none()
                } else {
                    self.update(Message::Send(ScooterCommand::SetUart(value)))
                }
            }
            Message::Gps(update) => {
                self.session.set_gps(update);
                Task::none()
            }
            Message::Insets(insets) => {
                self.insets = insets;
                Task::none()
            }
            Message::SystemScale(scale) => {
                self.scale = scale;
                Task::none()
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick),
            iced::time::every(Duration::from_secs(5)).map(|_| Message::ReconnectTick),
            crate::platform::transport_subscription(),
            iced_android::insets().map(Message::Insets),
            iced_android::system_scale().map(Message::SystemScale),
            iced::keyboard::listen().filter_map(|event| match event {
                iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Escape
                        | iced::keyboard::key::Named::BrowserBack,
                    ),
                    ..
                } => Some(Message::Back),
                _ => None,
            }),
        ])
    }
}
```

`App` gains a `pub mobile: bool` field (initialised to `cfg!(target_os = "android")`
in `with_transport`), so the `Entry` decision is testable on a desktop runner
instead of reading `cfg!` inline. Keep `#[derive(Debug, Clone, PartialEq)]` on
`Message` (the scanner tests compare messages).

`platform/mod.rs` gains:

```rust
use futures::StreamExt;
use iced::Subscription;

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
```

Add `use crate::message::Message;` and gate the Android-only imports if needed.
Task 13 adds the Android Back event producer; the keyboard path already works.

**Step 4: Re-run, run the app, commit**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cargo run -p zipp
cd ../..
git add app/rust/zipp
git commit -m "feat(ui): wire the update loop and subscriptions"
```

Manual check: the wall clock advances once a second, Entry logs a connection
error (no device at the URL), the log page opens from the status row, Back
returns, and the UART field sends nothing when disconnected (it logs
`Not connected`).

---

## Task 12: Add the desktop `btleplug` transport and platform wiring

**Files**

- Create: `app/rust/zipp/src/platform/desktop.rs`
- Modify: `app/rust/zipp/src/platform/mod.rs`

**Step 1: Write the failing helper tests**

In `desktop.rs`:

```rust
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
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp write_type
```

**Step 3: Implement the actor**

Use a dedicated thread with a current-thread tokio runtime, an unbounded
`TransportCommand` channel, and `broadcast`/`watch` senders. The shape:

```rust
use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::sync::{broadcast, mpsc, watch};
use zipp_core::transport::{DeviceId, ScanDevice, Transport, TransportCommand, TransportEvent};

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

pub fn write_type(without_response: bool) -> WriteType {
    if without_response { WriteType::WithoutResponse } else { WriteType::WithResponse }
}

pub fn device_name(local: Option<&str>, id: &str) -> String {
    local.filter(|name| !name.is_empty()).unwrap_or(id).to_string()
}
```

Actor state:

```rust
struct Actor {
    adapter: Option<Adapter>,
    peripherals: Vec<Peripheral>,
    connected: Option<Peripheral>,
    scan_until: Option<tokio::time::Instant>,
    events: broadcast::Sender<TransportEvent>,
}
```

`run` loop uses `tokio::select!` over `commands.recv()`, a 500 ms interval
while scanning, and a 15 s deadline. On startup:

- `Manager::new().await`; take the first adapter. If none, emit
  `AdapterState(false)` and keep serving commands (retry manager creation on
  the next `Reconnect`/`StartScan`).
- Emit `AdapterState(true)`.

Command handling:

- `StartScan { target }` / `Reconnect { target }`: `adapter.start_scan(ScanFilter::default()).await`;
  emit `ScanState(true)`; set `scan_until = now + 15s`; clear the cached
  peripherals. `Reconnect` behaves like `StartScan` on Linux because
  `btleplug` does not expose BlueZ's paired list.
- `StopScan`: `adapter.stop_scan().await`; emit `ScanState(false)`.
- Every 500 ms while scanning: `adapter.peripherals().await`; for each, read
  `properties()`, map `id().to_string()` to `DeviceId`, name from
  `properties.local_name`, rssi from `properties.rssi`; emit `Devices`.
  Do not filter by name in Rust; the session auto-connects only to `BScooter`,
  and the scanner list should show everything BlueZ reports (Flutter's
  `withNames` filtered, but the session filter is enough and more useful).
- At the 15 s deadline: stop scan and emit `ScanState(false)`.
- `Connect { id }`: stop scanning; find the peripheral by id; `connect()` with
  `tokio::time::timeout(Duration::from_secs(10), peripheral.connect())`; on
  error emit `Error(format!("Connection error: {error}"))`.
  Then `discover_services()`. For each characteristic:
  - if `properties` contains `NOTIFY` or `INDICATE`, `subscribe(&characteristic).await`;
  - if it contains `READ`, `read(&characteristic).await` and emit
    `ReadValue { uuid: uuid.to_string(), data }`.
  Emit `Connected { id, name }` using the peripheral's local name or the id.
  Keep the peripheral in `self.connected`; spawn a task that forwards
  `peripheral.notifications().await` into the event sender:
  `Notification { uuid: data.uuid.to_string(), data: data.value }`.
- `Write { service, characteristic, bytes }`: parse both UUIDs with
  `uuid::Uuid::parse_str` (add `uuid = "1"` to the desktop dependencies) and
  find the characteristic. Choose `WriteType::WithoutResponse` when
  `properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)`, otherwise
  `WithResponse`. On error emit `Error(format!("Command Failed: {error}"))`.
- `Read { .. }`: look up and `read`, emit `ReadValue`.
- `Disconnect`: `peripheral.disconnect().await`; clear the field; emit
  `Disconnected`.
- `RequestPermissions`: no-op on desktop.
- When the notification stream ends and the peripheral is no longer connected,
  emit `Disconnected`.
- No adapter or an off adapter: emit `AdapterState(false)` and log through the
  core; never panic. A failed command while disconnected emits
  `Error("Not connected")` for writes.

Add `uuid = "1"` under `[target.'cfg(not(target_os = "android"))'.dependencies]`.

**Step 4: Wire `platform/mod.rs`**

```rust
#[cfg(target_os = "linux")]
mod desktop;

pub fn ensure_transport() {
    TRANSPORT.get_or_init(|| {
        #[cfg(target_os = "linux")]
        {
            desktop::spawn()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Arc::new(NoopTransport::new())
        }
    });
}
```

Keep `NoopTransport` for non-Linux tests and future targets. `lang()` stays
`i18n::detect()` until Task 13 adds the Android locale override.

**Step 5: Re-run and manual desktop check**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
cargo run -p zipp
```

With no adapter, the log shows `Bluetooth is off` and the scanner stays empty;
with an adapter but no scooter, it shows nearby devices. Clicking a device
connects. This is the first hardware-dependent check; if you have no adapter,
record it in the technical notes.

**Step 6: Commit**

```sh
cd ../..
git add app/rust/zipp app/rust/Cargo.lock
git commit -m "feat(desktop): add btleplug transport"
```

---

## Task 13: Add the Android transport and JNI exports

**Files**

- Create: `app/rust/zipp/src/platform/android.rs`
- Create: `app/rust/zipp/src/jni.rs`
- Modify: `app/rust/zipp/src/lib.rs`
- Modify: `app/rust/zipp/src/platform/mod.rs`

**Step 1: Write the failing tests**

Pure helpers only; JNI and BLE need a device. In `android.rs`:

```rust
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
        assert_eq!(gps_status("searching"), zipp_core::gps::GpsStatus::Searching);
        assert_eq!(gps_status("active"), zipp_core::gps::GpsStatus::Active);
        assert_eq!(gps_status("junk"), zipp_core::gps::GpsStatus::Disabled);
    }
}
```

**Step 2: Run and watch them fail**

```sh
cd app/rust
cargo test -p zipp lang_from_tag gps_status
```

**Step 3: Implement the Android transport**

```rust
use std::sync::{Arc, OnceLock};

use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use tokio::sync::{broadcast, watch};
use zipp_core::gps::GpsStatus;
use zipp_core::i18n::Lang;
use zipp_core::transport::{Transport, TransportCommand, TransportEvent};

pub const BRIDGE_CLASS: &str = "io/bitshift/zipp/ZippBridge";

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

pub fn lang_from_tag(tag: &str) -> Lang {
    Lang::from_tag(tag)
}

pub fn current_lang() -> Lang {
    LANG.get().copied().unwrap_or_default()
}

pub fn gps_status(value: &str) -> GpsStatus {
    match value {
        "denied" => GpsStatus::Denied,
        "searching" => GpsStatus::Searching,
        "active" => GpsStatus::Active,
        _ => GpsStatus::Disabled,
    }
}
```

`JNI_OnLoad` caches the VM and class:

```rust
pub fn on_load(vm: JavaVM, env: &mut JNIEnv) {
    let class = env.find_class(BRIDGE_CLASS).expect("ZippBridge class");
    let bridge = env.new_global_ref(class).expect("global ZippBridge");
    let _ = VM.set(vm);
    let _ = BRIDGE.set(bridge);
}
```

`install(locale: &str)` creates the broadcast/watch senders, stores the
transport and language, and returns an error if the bridge class was not
cached (log it, never panic):

```rust
pub fn install(env: &mut JNIEnv, locale: &str) -> Result<(), String> {
    let vm = env.get_java_vm().map_err(|e| e.to_string())?;
    let _ = LANG.set(lang_from_tag(locale));
    let (events, _) = broadcast::channel(256);
    let (wifi, _) = watch::channel(false);
    let (back, _) = broadcast::channel(8);
    let _ = BACK.set(back);
    let bridge = BRIDGE.get().ok_or("ZippBridge class not cached")?.clone();
    let transport = Arc::new(AndroidTransport { vm, bridge, events, wifi });
    let _ = TRANSPORT.set(transport);
    Ok(())
}

pub fn transport() -> Option<Arc<AndroidTransport>> {
    TRANSPORT.get().cloned()
}

pub fn back_sender() -> Option<broadcast::Sender<()>> {
    BACK.get().cloned()
}
```

`send` attaches the current thread and calls the static Kotlin methods:

```rust
fn call(&self, method: &str, signature: &str, args: &[JValue<'_>]) {
    let Ok(mut env) = self.vm.attach_current_thread() else { return; };
    if let Err(error) = env.call_static_method(self.bridge.as_obj(), method, signature, args) {
        let _ = env.exception_clear();
        log::warn!("ZippBridge.{method}: {error}");
    }
}
```

Map commands:

| Command | Kotlin call |
| :--- | :--- |
| `Reconnect { target }` | `reconnect(String)` |
| `StartScan { target }` | `startScan(String)` |
| `StopScan` | `stopScan()` |
| `Connect { id }` | `connect(String)` |
| `Disconnect` | `disconnect()` |
| `Write { service, characteristic, bytes }` | `write(String, String, ByteArray)` |
| `Read { .. }` | `read(String, String)` |
| `RequestPermissions` | `requestPermissions()` |

Signatures: `(Ljava/lang/String;)V`, `()V`,
`(Ljava/lang/String;Ljava/lang/String;[B)V`.

Callbacks (called by `jni.rs`) emit into the broadcast sender:

```rust
pub fn emit(event: TransportEvent) {
    if let Some(transport) = TRANSPORT.get() {
        let _ = transport.events.send(event);
    }
}

pub fn emit_wifi(value: bool) {
    if let Some(transport) = TRANSPORT.get() {
        let _ = transport.wifi.send(value);
        let _ = transport.events.send(TransportEvent::Wifi(value));
    }
}

pub fn emit_gps_status(value: &str) {
    // GPS bypasses TransportEvent; jni.rs forwards a Message through a
    // dedicated broadcast channel. See gps_sender.
}

pub fn emit_gps_speed(_mps: f64) {}
```

Keep GPS on its own channel so it reaches `Message::Gps`:

```rust
static GPS: OnceLock<broadcast::Sender<zipp_core::gps::GpsUpdate>> = OnceLock::new();

pub fn gps_sender() -> broadcast::Sender<zipp_core::gps::GpsUpdate> {
    GPS.get_or_init(|| broadcast::channel(64).0).clone()
}
```

Create `GPS` in `install` too, and have `emit_gps_status`/`emit_gps_speed` send
`GpsUpdate { status, speed_mps }` (status from the last status event, speed
optional).

`platform/mod.rs` on Android:

```rust
#[cfg(target_os = "android")]
mod android;

pub fn ensure_transport() {
    TRANSPORT.get_or_init(|| {
        #[cfg(target_os = "android")]
        {
            android::transport()
                .map(|transport| transport as Arc<dyn Transport>)
                .unwrap_or_else(|| Arc::new(NoopTransport::new()))
        }
        #[cfg(target_os = "linux")]
        {
            desktop::spawn()
        }
        #[cfg(not(any(target_os = "android", target_os = "linux")))]
        {
            Arc::new(NoopTransport::new())
        }
    });
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
```

Add `back_subscription()` and `gps_subscription()` to `App::subscription`.

**Step 4: Implement `jni.rs`**

Exports (all `#[unsafe(no_mangle)]`, `extern "system"`):

```text
Java_io_bitshift_zipp_ZippBridge_nativeInit(env, this, locale)
Java_io_bitshift_zipp_ZippBridge_nativeBack(env, this)
Java_io_bitshift_zipp_ZippBridge_onScanResult(env, this, address, name, rssi)
Java_io_bitshift_zipp_ZippBridge_onScanState(env, this, scanning)
Java_io_bitshift_zipp_ZippBridge_onAdapterState(env, this, on)
Java_io_bitshift_zipp_ZippBridge_onConnectionState(env, this, address, connected)
Java_io_bitshift_zipp_ZippBridge_onNotification(env, this, uuid, bytes)
Java_io_bitshift_zipp_ZippBridge_onRead(env, this, uuid, bytes)
Java_io_bitshift_zipp_ZippBridge_onWifiState(env, this, connected)
Java_io_bitshift_zipp_ZippBridge_onGpsStatus(env, this, status)
Java_io_bitshift_zipp_ZippBridge_onGpsSpeed(env, this, mps)
Java_io_bitshift_zipp_ZippBridge_onError(env, this, message)
```

`JNI_OnLoad` calls `android::on_load`, installs `android_logger` with tag
`zipp`, and returns `JNI_VERSION_1_6`.

`nativeInit` reads the `JString` locale, initializes logging, calls
`android::install`, and on success calls `ZippBridge.registerWifi()` and
`ZippBridge.startGps()` through the transport? No: Kotlin `MainActivity`
calls those directly after `nativeInit`; Rust only needs the locale and
transport. Do not duplicate.

`nativeBack` sends on the BACK broadcast.

Callback conversion rules:

- `JString` -> `String` with `env.get_string(&value).map(Into::into).unwrap_or_default()`.
- `jbyteArray` -> `Vec<u8>` with `env.convert_byte_array(&array).unwrap_or_default()`.
- `jboolean` -> `!= 0`.
- `jint` -> `i32` then `i16` for RSSI.
- `jdouble` -> `f64`.
- Clear any pending exception after a failed conversion and return.

Callback mapping:

| JNI callback | Core event |
| :--- | :--- |
| `onScanResult` | accumulate? No: emit one `Devices` list on each callback is expensive. Kotlin filters by name and the list is small. Have `jni.rs` push into a `Mutex<BTreeMap<String, ScanDevice>>` and emit a snapshot. Clear the map on `onScanState(true)`. |
| `onScanState` | `ScanState(bool)` |
| `onAdapterState` | `AdapterState(bool)` |
| `onConnectionState` | connected ? `Connected { id, name }` : `Disconnected`. The bridge sends the address; the name is the target (`BScooter`) because the filter only accepts that name. Use `target_device_name()`. |
| `onNotification` | `Notification { uuid, data }` |
| `onRead` | `ReadValue { uuid, data }` |
| `onWifiState` | `Wifi(bool)` |
| `onGpsStatus` | GPS channel `GpsUpdate` |
| `onGpsSpeed` | GPS channel `GpsUpdate` with `Active` and the speed |
| `onError` | `Error(String)` |

`lib.rs` changes:

```rust
#[cfg(target_os = "android")]
mod jni;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: iced_android::AndroidApp) {
    iced_android::init(app);
    if let Err(error) = run() {
        log::error!("iced failed: {error}");
    }
}
```

`android_logger` initialization belongs in `nativeInit` (before the window
starts) so a crash during `android_main` is logged.

**Step 5: Compile-check the Android target**

```sh
cd app/rust
cargo fmt --all
cargo test -p zipp
rustup target list --installed | grep aarch64-linux-android
cargo check -p zipp --target aarch64-linux-android
```

If the target is missing, `rustup target add aarch64-linux-android` first.
`cargo check` does not link, so it works without the NDK; Task 16 adds the
linker environment for the APK.

**Step 6: Commit**

```sh
cd ../..
git add app/rust/zipp app/rust/Cargo.lock
git commit -m "feat(android): add JNI transport and callbacks"
```

---

## Task 14: Replace the Android Gradle shell

**Files**

- Delete/replace: `app/android/settings.gradle.kts`, `app/android/build.gradle.kts`,
  `app/android/app/build.gradle.kts`, `app/android/gradle.properties`,
  `app/android/gradle/wrapper/gradle-wrapper.properties`
- Copy: `app/android/gradlew`, `app/android/gradlew.bat`,
  `app/android/gradle/wrapper/gradle-wrapper.jar` from `pige`
- Create: `app/android/app/src/main/res/values/iced.xml`
- Create: `app/android/app/src/main/res/values/strings.xml`
- Replace: `app/android/app/src/main/AndroidManifest.xml`
- Delete: Flutter launch themes and debug/profile manifests:
  `app/android/app/src/main/res/values/styles.xml`,
  `app/android/app/src/main/res/values-night/`,
  `app/android/app/src/main/res/values-v31/`,
  `app/android/app/src/main/res/values-night-v31/`,
  `app/android/app/src/main/res/drawable/launch_background.xml`,
  `app/android/app/src/main/res/drawable-v21/`,
  `app/android/app/src/debug/`, `app/android/app/src/profile/`
- Create: `app/scripts/build.sh`, `app/scripts/version.sh`

**Step 1: Copy the wrapper**

```sh
cp /home/bronson/Projects/pige/android/gradlew app/android/
cp /home/bronson/Projects/pige/android/gradlew.bat app/android/
cp /home/bronson/Projects/pige/android/gradle/wrapper/gradle-wrapper.jar \
   app/android/gradle/wrapper/
cp /home/bronson/Projects/pige/android/gradle/wrapper/gradle-wrapper.properties \
   app/android/gradle/wrapper/
chmod +x app/android/gradlew
```

**Step 2: Write the Gradle files**

`app/android/settings.gradle.kts`:

```kotlin
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

plugins {
    id("com.android.application") version "9.4.1" apply false
    id("org.jetbrains.kotlin.android") version "2.4.0" apply false
}

include(":app")
```

`app/android/build.gradle.kts`:

```kotlin
allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}
subprojects {
    project.evaluationDependsOn(":app")
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
```

`app/android/app/build.gradle.kts`:

```kotlin
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val keystoreProperties = Properties().apply {
    val file = rootProject.file("keystore.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
val hasSharedKey = keystoreProperties.getProperty("storeFile") != null

android {
    namespace = "io.bitshift.zipp"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    sourceSets.getByName("main").java.setSrcDirs(listOf(
        "src/main/kotlin", providers.gradleProperty("icedAndroidJava").get()
    ))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "io.bitshift.zipp"
        minSdk = 34
        targetSdk = 36
        versionCode = providers.gradleProperty("zippVersionCode").getOrElse("1").toInt()
        versionName = providers.gradleProperty("zippVersionName").getOrElse("0.0.0")
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        if (hasSharedKey) {
            create("shared") {
                storeFile = rootProject.file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            signingConfig = if (hasSharedKey) {
                signingConfigs.getByName("shared")
            } else {
                signingConfigs.getByName("debug")
            }
        }
        release {
            signingConfig = if (hasSharedKey) {
                signingConfigs.getByName("shared")
            } else {
                signingConfigs.getByName("debug")
            }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}
```

`app/android/gradle.properties`:

```properties
org.gradle.jvmargs=-Xmx8G -XX:MaxMetaspaceSize=4G -XX:ReservedCodeCacheSize=512m -XX:+HeapDumpOnOutOfMemoryError
org.gradle.configuration-cache=true
android.useAndroidX=true
android.newDsl=false
android.builtInKotlin=false
```

Wrapper properties (if the copied file still says 9.8.1, leave it):

```properties
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-9.8.1-all.zip
```

**Step 3: Write the manifest and resources**

`AndroidManifest.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET"/>
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>
    <uses-permission android:name="android.permission.BLUETOOTH"/>
    <uses-permission android:name="android.permission.BLUETOOTH_ADMIN"/>
    <uses-permission android:name="android.permission.BLUETOOTH_SCAN"/>
    <uses-permission android:name="android.permission.BLUETOOTH_CONNECT"/>
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION"/>

    <uses-feature android:name="android.hardware.bluetooth_le" android:required="true"/>

    <application
        android:label="@string/app_name"
        android:allowBackup="false"
        android:icon="@mipmap/ic_launcher"
        android:usesCleartextTraffic="true">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:launchMode="singleTask"
            android:enableOnBackInvokedCallback="true"
            android:theme="@style/IcedTheme"
            android:configChanges="orientation|keyboardHidden|keyboard|screenSize|smallestScreenSize|locale|layoutDirection|fontScale|screenLayout|density|uiMode"
            android:hardwareAccelerated="true"
            android:windowSoftInputMode="adjustResize">
            <meta-data android:name="android.app.lib_name" android:value="zipp"/>
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
```

`res/values/iced.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="IcedTheme" parent="@android:style/Theme.Material.NoActionBar">
        <item name="android:windowBackground">@android:color/black</item>
        <item name="android:windowActionModeOverlay">true</item>
        <item name="android:windowNoTitle">true</item>
        <item name="android:statusBarColor">@android:color/transparent</item>
        <item name="android:navigationBarColor">@android:color/transparent</item>
        <item name="android:enforceStatusBarContrast">false</item>
        <item name="android:enforceNavigationBarContrast">false</item>
        <item name="android:windowLightStatusBar">false</item>
        <item name="android:windowLightNavigationBar">false</item>
    </style>
</resources>
```

`res/values/strings.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Zipp</string>
</resources>
```

Keep the existing `mipmap-*/ic_launcher.png` files; delete the Flutter
`mipmap.png` extras only if they are unused.

**Step 4: Add the Android build scripts**

`app/scripts/version.sh`:

```bash
#!/usr/bin/env bash
# Print the zipp package version. Single source of truth: rust/Cargo.toml.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cargo metadata --locked --format-version 1 --no-deps \
    --manifest-path "$ROOT/rust/Cargo.toml" \
    | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "zipp"))'
```

`app/scripts/build.sh` is `pige/scripts/build.sh` with these substitutions:

- `pige` -> `zipp`
- `-PqlaunchVersionName` -> `-PzippVersionName`
- `-PqlaunchVersionCode` -> `-PzippVersionCode`
- `libpige.so` -> `libzipp.so`
- drop `--features provider-google` (a pige-only feature the `zipp` crate
  does not have)

Keep the JDK detection, `ANDROID_HOME`/`ANDROID_NDK_HOME` defaults, NDK clang
linker, `CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS`, `cargo metadata`
resolution of `iced_android`'s `java` directory, and the version-code form
`V_MAJOR * 10000 + V_MINOR * 100 + V_PATCH`.

Make both executable:

```sh
chmod +x app/scripts/build.sh app/scripts/version.sh
bash -n app/scripts/build.sh
bash -n app/scripts/version.sh
bash app/scripts/version.sh
```

The last command prints `2026.11.8`.

**Step 5: Commit**

The APK cannot build until Task 15 adds `MainActivity.kt`; that is expected.

```sh
git add -A app/android app/scripts
git commit -m "feat(android): replace the Flutter Gradle shell"
```

---

## Task 15: Add the Kotlin `ZippBridge` and `MainActivity`

**Files**

- Delete: `app/android/app/src/main/kotlin/io/bitshift/zipp/MainActivity.kt`
- Create: `app/android/app/src/main/kotlin/io/bitshift/zipp/MainActivity.kt`
- Create: `app/android/app/src/main/kotlin/io/bitshift/zipp/ZippBridge.kt`

**Step 1: Write `MainActivity.kt`**

```kotlin
package io.bitshift.zipp

import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.os.Bundle
import io.github.santuzius.icedandroid.IcedActivity
import java.util.Locale

class MainActivity : IcedActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        System.loadLibrary("zipp")
        ZippBridge.install(this, applicationContext)
        ZippBridge.nativeInit(Locale.getDefault().language)
        super.onCreate(savedInstanceState)
        window.setFormat(PixelFormat.RGBA_8888)
        ZippBridge.requestPermissions()
        ZippBridge.registerWifi()
        ZippBridge.startGps()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        ZippBridge.onPermissionResult(
            grantResults.isNotEmpty() &&
                grantResults.all { it == PackageManager.PERMISSION_GRANTED }
        )
    }

    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        ZippBridge.nativeBack()
    }

    override fun onDestroy() {
        ZippBridge.shutdown()
        super.onDestroy()
    }
}
```

**Step 2: Write `ZippBridge.kt`**

Use one `object`; declare the Rust-facing callbacks as `external fun` inside it
(they are instance methods on the singleton, which is why the Rust symbols take
a `JObject`). `install` stores the activity weakly and the application context.

```kotlin
package io.bitshift.zipp

import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Handler
import android.os.Looper
import java.lang.ref.WeakReference
import java.util.UUID

object ZippBridge {
    private const val CCCD = "00002902-0000-1000-8000-00805f9b34fb"
    private const val REQUEST_PERMISSIONS = 1
    private const val SCAN_TIMEOUT_MS = 15_000L
    private const val CONNECT_TIMEOUT_MS = 10_000L

    private var activity: WeakReference<MainActivity>? = null
    private lateinit var context: Context
    private var bluetoothManager: BluetoothManager? = null
    private var adapter: BluetoothAdapter? = null
    private var scanner: BluetoothLeScanner? = null
    private var gatt: BluetoothGatt? = null
    private var characteristics = mutableMapOf<UUID, BluetoothGattCharacteristic>()
    private var targetName = "BScooter"
    private var pendingScanTarget: String? = null
    private var scanTimeout: Runnable? = null
    private var connectTimeout: Runnable? = null
    private var locationManager: LocationManager? = null
    private var locationListener: LocationListener? = null
    private var connectivity: ConnectivityManager? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private val handler = Handler(Looper.getMainLooper())

    external fun nativeInit(locale: String)
    external fun nativeBack()
    external fun onScanResult(address: String, name: String, rssi: Int)
    external fun onScanState(scanning: Boolean)
    external fun onAdapterState(on: Boolean)
    external fun onConnectionState(address: String, connected: Boolean)
    external fun onNotification(uuid: String, bytes: ByteArray)
    external fun onRead(uuid: String, bytes: ByteArray)
    external fun onWifiState(connected: Boolean)
    external fun onGpsStatus(status: String)
    external fun onGpsSpeed(mps: Double)
    external fun onError(message: String)

    @JvmStatic
    fun install(activity: MainActivity, appContext: Context) {
        this.activity = WeakReference(activity)
        context = appContext.applicationContext
        bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
        adapter = bluetoothManager?.adapter
        scanner = adapter?.bluetoothLeScanner
        connectivity = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    }
    // ... command and callback methods follow
}
```

Implement the commands:

```kotlin
@JvmStatic
fun requestPermissions() {
    val activity = activity?.get() ?: return
    val needed = mutableListOf<String>()
    for (permission in listOf(
        Manifest.permission.BLUETOOTH_SCAN,
        Manifest.permission.BLUETOOTH_CONNECT,
        Manifest.permission.ACCESS_FINE_LOCATION,
    )) {
        if (activity.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
            needed += permission
        }
    }
    if (needed.isNotEmpty()) {
        activity.requestPermissions(needed.toTypedArray(), REQUEST_PERMISSIONS)
    }
}

fun onPermissionResult(granted: Boolean) {
    if (!granted) {
        onError("Permissions denied for scanning")
        return
    }
    pendingScanTarget?.let { target ->
        pendingScanTarget = null
        startScan(target)
    }
    startGps()
}

@JvmStatic
fun startScan(target: String) {
    targetName = target
    if (!hasScanPermissions()) {
        pendingScanTarget = target
        requestPermissions()
        return
    }
    // `onAdapterState(false)` already makes the core log the single
    // "Bluetooth is off" line (as Flutter does); a second `onError` would
    // duplicate it on every reconnect tick.
    val bleAdapter = adapter ?: run {
        onAdapterState(false)
        return
    }
    if (!bleAdapter.isEnabled) {
        onAdapterState(false)
        return
    }
    onAdapterState(true)
    stopScan()
    try {
        val filter = ScanFilter.Builder().setDeviceName(target).build()
        val settings = ScanSettings.Builder()
            .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
            .build()
        scanner?.startScan(listOf(filter), settings, scanCallback)
        onScanState(true)
        scanTimeout = Runnable { stopScan() }.also {
            handler.postDelayed(it, SCAN_TIMEOUT_MS)
        }
    } catch (error: Exception) {
        onError("Scan Error: $error")
    }
}

@JvmStatic
fun stopScan() {
    scanTimeout?.let { handler.removeCallbacks(it) }
    scanTimeout = null
    try {
        scanner?.stopScan(scanCallback)
    } catch (_: Exception) {
    }
    onScanState(false)
}

@JvmStatic
fun reconnect(target: String) {
    val bonded = adapter?.bondedDevices?.firstOrNull { it.name == target }
    if (bonded != null) {
        connect(bonded.address)
    } else {
        startScan(target)
    }
}

@JvmStatic
fun connect(address: String) {
    val device = adapter?.getRemoteDevice(address) ?: return
    stopScan()
    disconnect()
    gatt = device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
    connectTimeout = Runnable {
        onError("Connection error: timeout")
        disconnect()
    }.also { handler.postDelayed(it, CONNECT_TIMEOUT_MS) }
}

@JvmStatic
fun disconnect() {
    connectTimeout?.let { handler.removeCallbacks(it) }
    connectTimeout = null
    characteristics.clear()
    gatt?.disconnect()
    gatt?.close()
    gatt = null
}

@JvmStatic
fun write(service: String, characteristic: String, bytes: ByteArray) {
    val gatt = gatt ?: return
    val char = characteristics[UUID.fromString(characteristic)] ?: return
    val withoutResponse =
        char.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0
    val writeType = if (withoutResponse) {
        BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
    } else {
        BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
    }
    try {
        gatt.writeCharacteristic(char, bytes, writeType)
    } catch (error: Exception) {
        onError("Command Failed: $error")
    }
}

@JvmStatic
fun read(service: String, characteristic: String) {
    val gatt = gatt ?: return
    val char = characteristics[UUID.fromString(characteristic)] ?: return
    try {
        gatt.readCharacteristic(char)
    } catch (error: Exception) {
        onError("Command Failed: $error")
    }
}
```

Implement `scanCallback`:

```kotlin
private val scanCallback = object : ScanCallback() {
    override fun onScanResult(callbackType: Int, result: ScanResult) {
        val name = result.scanRecord?.deviceName
            ?: result.device.name
            ?: ""
        onScanResult(result.device.address, name, result.rssi)
    }

    override fun onScanFailed(errorCode: Int) {
        onError("Scan Error: $errorCode")
        onScanState(false)
    }
}
```

Implement `gattCallback`:

- `onConnectionStateChange`: if `status == GATT_SUCCESS && newState == STATE_CONNECTED`,
  cancel the connect timeout, `gatt.discoverServices()`, and
  `onConnectionState(device.address, true)`.
  If `newState == STATE_DISCONNECTED`, cancel the timeout, clear the map, close
  and null the gatt, and `onConnectionState(device.address, false)`.
  On any other status, `onError("Connection error: status $status")`.
- `onServicesDiscovered`: for every characteristic, store it in the map; when
  `properties & (PROPERTY_NOTIFY or PROPERTY_INDICATE) != 0`, call
  `setCharacteristicNotification(char, true)` and write the CCCD
  (`ENABLE_NOTIFICATION_VALUE` for notify, `ENABLE_INDICATION_VALUE` for
  indicate); when `properties & PROPERTY_READ != 0`, call `readCharacteristic`.
- `onCharacteristicChanged(gatt, char)` and the API 33 overload
  `onCharacteristicChanged(gatt, char, value)` -> `onNotification(char.uuid.toString(), value)`.
- `onCharacteristicRead(gatt, char, value, status)` and the older overload ->
  `onRead(char.uuid.toString(), value)`.
- `onCharacteristicWrite` -> on failure `onError("Command Failed: status $status")`.

Implement GPS:

```kotlin
@JvmStatic
fun startGps() {
    if (context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) !=
        PackageManager.PERMISSION_GRANTED) {
        onGpsStatus("denied")
        return
    }
    val manager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
    locationManager = manager
    if (!manager.isProviderEnabled(LocationManager.GPS_PROVIDER)) {
        onGpsStatus("disabled")
        return
    }
    val listener = object : LocationListener {
        override fun onLocationChanged(location: Location) {
            onGpsStatus("active")
            onGpsSpeed(location.speed.toDouble())
        }
        override fun onProviderEnabled(provider: String) {}
        override fun onProviderDisabled(provider: String) {
            onGpsStatus("disabled")
        }
        @Deprecated("Deprecated in Java")
        override fun onStatusChanged(provider: String?, status: Int, extras: android.os.Bundle?) {}
    }
    locationListener = listener
    try {
        manager.requestLocationUpdates(
            LocationManager.GPS_PROVIDER, 0L, 0f, listener, Looper.getMainLooper(),
        )
        onGpsStatus("searching")
    } catch (error: Exception) {
        onGpsStatus("disabled")
        onError("GPS Error: $error")
    }
}

@JvmStatic
fun stopGps() {
    locationListener?.let { locationManager?.removeUpdates(it) }
    locationListener = null
    locationManager = null
}
```

Implement Wi-Fi:

```kotlin
@JvmStatic
fun registerWifi() {
    val manager = connectivity ?: return
    networkCallback?.let { manager.unregisterNetworkCallback(it) }
    val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) = report(manager, network)
        override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) =
            report(manager, network)
        override fun onLost(network: Network) {
            onWifiState(false)
        }
    }
    networkCallback = callback
    manager.registerDefaultNetworkCallback(callback)
    val current = manager.activeNetwork
    onWifiState(
        current != null &&
            manager.getNetworkCapabilities(current)
                ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
    )
}

private fun report(manager: ConnectivityManager, network: Network) {
    val wifi = manager.getNetworkCapabilities(network)
        ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
    onWifiState(wifi)
}
```

Implement `shutdown()`: `stopScan()`, `disconnect()`, `stopGps()`,
`networkCallback?.let { connectivity?.unregisterNetworkCallback(it) }`, clear
the activity.

`hasScanPermissions()` checks `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT` and
`ACCESS_FINE_LOCATION`.

**Step 3: Compile the Kotlin**

This needs the Rust `.so`; run Task 16's build. A Kotlin-only syntax check is
not available without Gradle. Keep the file formatted consistently with
`pige`.

**Step 4: Commit**

```sh
git add app/android/app/src/main/kotlin
git commit -m "feat(android): add ZippBridge BLE, GPS and Wi-Fi bridge"
```

---

## Task 16: Build and run the Android app

**Files**

- Modify: `app/scripts/build.sh` only if the first build exposes a path issue
- Add: `app/android/app/src/main/jniLibs/` to `.gitignore` (Task 19)

**Step 1: Build the debug APK**

```sh
cd /home/bronson/Projects/zipp/app
bash scripts/build.sh debug
```

Expected output ends with `BUILD SUCCESSFUL` and
`build/app/outputs/apk/debug/app-debug.apk`.

If Gradle fails:

- `JAVA_HOME` must contain `bin/javac`; `/opt/jbr-21` is the intended JDK.
- `ANDROID_HOME` defaults to `$HOME/Android/Sdk`; set it explicitly if the SDK
  lives elsewhere.
- `ANDROID_NDK_HOME` must be `28.2.13676358`.
- The linker line must use `aarch64-linux-android34-clang`.
- The 16 KiB page flag must be in `CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS`.

**Step 2: Install and launch**

```sh
adb devices
adb install -r build/app/outputs/apk/debug/app-debug.apk
adb shell am start -W -n io.bitshift.zipp/.MainActivity
adb logcat -s zipp
```

**Step 3: Work the manual Android checklist**

1. The permission dialog asks for nearby devices and precise location; grant.
2. The scanner shows `Scanning...`, finds `BScooter`, and auto-connects.
3. The dashboard shows live speed, battery, temperature, clock, odometer and
   cruise bars.
4. Power, Alarm, Lights, Sport, Cruise Up, Cruise Down and Horn all write; the
   scooter reacts.
5. The status row opens the log page; UART RX lines appear; typed UART text
   reaches the scooter.
6. GPS shows searching, then an active speed.
7. Rotate, background/foreground, and return; the window survives.
8. `adb logcat -s zipp` has no Rust panic, JNI exception or ANR.
9. Deny permissions once and confirm `Permissions denied for scanning` appears
   instead of a crash.

Fix defects in place and commit each fix:

```sh
git add -A
git commit -m "fix(android): <what the device revealed>"
```

**Step 4: Build the release APK**

```sh
bash scripts/build.sh release
ls -l build/app/outputs/apk/release/app-release.apk
```

The APK is signed with the local debug key unless
`app/android/keystore.properties` exists; that is the intended fallback.

---

## Task 17: Run the cross-platform gate and the desktop checklist

**Files**

- Fix any files the gate exposes

**Step 1: Format, lint and test**

```sh
cd app/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p zipp --target aarch64-linux-android
```

`--workspace` still includes the old Flutter package, so if the old crate has a
pre-existing warning, use `cargo clippy -p zipp_core -p zipp --all-targets -- -D warnings`
and record the exception. Task 19 removes the old crate and makes the full
workspace command clean.

**Step 2: Desktop checklist**

1. `cd app && ./run.sh` (add `run.sh` in Task 20; until then
   `cd app/rust && cargo run -p zipp --bin zipp`).
2. The dashboard matches the Flutter layout at 420x820: 480 dp dashboard,
   five rows, fixed sizes and colours.
3. Resize the window; the dashboard stays 480 and the scanner takes the rest.
4. Entry sends a GET; `adb logcat`/stdout shows the status or the connection
   error.
5. The scanner shows `No devices found`, `Bluetooth is off`, or real devices
   without crashing.
6. With a scooter powered on, scan/connect/notify/read/write all work.
7. Open the log page, clear it, type a UART command, send it, and go back.

**Step 3: Commit any fixes**

```sh
cd /home/bronson/Projects/zipp
git add -A
git commit -m "fix: cross-platform verification findings"
```

If there are no changes, do not create an empty commit.

---

## Task 18: Write the architecture and technical notes

**Files**

- Create: `app/ARCHITECTURE.md`
- Create: `app/docs/technical-notes.md`

**Step 1: Write `app/ARCHITECTURE.md`**

Use `qlaunch/ARCHITECTURE.md` as the shape. Cover:

- The two crates and their dependency direction (`zipp` -> `zipp_core`; never the
  reverse).
- A data-flow diagram:

  ```text
  button/input -> Message -> App::update -> session mutation + Task
    -> Transport::send -> platform actor (btleplug or JNI)
    -> TransportEvent -> broadcast -> subscription -> Message::Transport
    -> session.apply -> view
  ```

- The `Transport` trait, `TransportCommand`, `TransportEvent`, and why the
  Android OS APIs live in Kotlin while UUID parsing and command bytes live in
  Rust.
- Android lifecycle: `nativeInit` installs the transport and locale before
  `android_main`; the iced window is never closed on background; Back is
  dispatched through both `nativeBack` and the `BrowserBack` key.
- Page/state ownership: `ScooterSession` is pure, `App` owns the page, UART
  input, insets and scale.
- The single deliberate UI deviation: instant page switch instead of a 300 ms
  slide.

**Step 2: Write `app/docs/technical-notes.md`**

Start with these headings and fill the measured results during Tasks 16–21:

```markdown
# Zipp technical notes

## Environment
- Host:
- Rust:
- JDK:
- NDK:
- Device / Android:

## Build measurements
- First `cargo check -p zipp`:
- `scripts/build.sh debug`:
- Release APK size:

## Device results
- BLE scan / auto-connect:
- Notify / read / write:
- GPS:
- Wi-Fi / Entry:
- Rotation / background:
- Logcat panic or JNI exception:

## Deviations
- Dashboard/log switch is an instant swap instead of Flutter's 300 ms slide.

## Desktop Chinese font
- Detected font path (or "none; tofu"):
```

**Step 3: Commit**

```sh
git add app/ARCHITECTURE.md app/docs/technical-notes.md
git commit -m "docs: describe the iced architecture"
```

---

## Task 19: Remove Flutter and the bridge from the active tree

**Files**

- Delete the whole Flutter layer (see the list)
- Replace `app/rust/Cargo.toml` with the pure workspace
- Delete `app/rust/src/`, `app/rust/Cargo.lock` (regenerate)
- Rewrite `app/.gitignore`
- Keep `app/assets/app_icon.{png,svg}` and the Android launcher PNGs

**Step 1: Confirm the removal inventory**

```sh
cd /home/bronson/Projects/zipp
git ls-files app | grep -E '^(app/lib/|app/test/|app/integration_test/|app/test_driver/|app/(ios|linux|macos|windows|web)/|app/rust_builder/|app/pubspec|app/analysis_options|app/.metadata|app/flutter_rust_bridge|app/run.sh|app/build.sh)'
```

Every path that appears is part of the Flutter app and must go, except
`app/run.sh` and `app/build.sh`, which are rewritten in Task 20.

**Step 2: Delete the Flutter tree**

```sh
git rm -r app/lib app/test app/integration_test app/test_driver
git rm -r app/ios app/linux app/macos app/windows app/web app/rust_builder
git rm app/pubspec.yaml app/pubspec.lock app/analysis_options.yaml \
       app/.metadata app/flutter_rust_bridge.yaml
git rm -r app/rust/src
rm -f app/rust/Cargo.lock
```

Also delete any Flutter Gradle files that Task 14 did not already remove:

```sh
rm -rf app/android/app/src/debug app/android/app/src/profile
rm -f app/android/app/src/main/res/values/styles.xml
rm -rf app/android/app/src/main/res/values-night \
       app/android/app/src/main/res/values-v31 \
       app/android/app/src/main/res/values-night-v31 \
       app/android/app/src/main/res/drawable \
       app/android/app/src/main/res/drawable-v21
git add -A app/android
```

Do **not** delete `app/android/app/src/main/res/mipmap-*`: those are the launcher
icons.

**Step 3: Make `app/rust/Cargo.toml` the pure workspace**

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

**Step 4: Rewrite `app/.gitignore`**

```gitignore
# Rust
/rust/target/
**/*.rs.bk

# Android / Gradle
/android/.gradle/
/android/local.properties
/android/.kotlin/
/android/keystore.properties
/android/keystore/
/android/app/src/main/jniLibs/
/build/
/android/captures/
*.iml
.idea/

# Release artefacts
/dist/

# Desktop
.cache/
```

Keep the existing Android-specific entries if they are still useful. Do not
ignore `app/assets/` or `app/android/app/src/main/res/`.

**Step 5: Regenerate, test and grep**

```sh
cd app/rust
cargo generate-lockfile
cargo test --workspace
cd ../..
grep -RinE 'flutter|dart|pubspec|flutter_rust_bridge' \
  --exclude-dir=old --exclude-dir=.git --exclude-dir=target \
  app
```

The grep must only find historical prose in
`app/docs/superpowers/` (the design and this plan). No build input may contain a
Flutter/Dart reference. The `old/` branch is excluded because it is the archive.

**Step 6: Commit**

```sh
git add -A app
git commit -m "chore: remove Flutter from the active tree"
```

---

## Task 20: Rewrite the scripts, README and root docs

**Files**

- Create: `app/run.sh`
- Create: `app/run_adb.sh`
- Create/rewrite: `app/scripts/release.sh`, `app/scripts/publish.sh`
- Rewrite: `app/README.md`
- Rewrite: `README.md` (repository root)
- Create: `/.gitignore` (repository root)
- Fill: `app/docs/technical-notes.md`

**Step 1: `app/run.sh`**

```bash
#!/usr/bin/env bash
# Run Zipp on the desktop. Extra args go to cargo (e.g. ./run.sh --release).
set -euo pipefail
cd "$(dirname "$0")/rust"

if [[ -z "${XDG_RUNTIME_DIR:-}" ]]; then
  export XDG_RUNTIME_DIR="/run/user/$(id -u)"
fi
if [[ -z "${WAYLAND_DISPLAY:-}" && -S "$XDG_RUNTIME_DIR/wayland-1" ]]; then
  export WAYLAND_DISPLAY=wayland-1
fi

exec cargo run -p zipp --bin zipp "$@"
```

**Step 2: `app/run_adb.sh`**

```bash
#!/usr/bin/env bash
# Build, install, launch and tail the log for the iced Android app.
# Usage: run_adb.sh [debug|release]   (defaults to debug)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
MODE="${1:-debug}"
case "$MODE" in debug|release) ;; *) echo "Usage: $0 [debug|release]" >&2; exit 2;; esac

if ! command -v adb >/dev/null 2>&1; then
  echo "adb not found in PATH." >&2
  exit 1
fi
if ! adb get-state 1>/dev/null 2>&1; then
  echo "No Android device attached. Connect one and enable USB debugging." >&2
  exit 1
fi
adb wait-for-device
SERIAL="$(adb get-serialno)"
MODEL="$(adb -s "$SERIAL" shell getprop ro.product.model | tr -d '\r')"
echo "Device: $SERIAL ($MODEL)"

export JAVA_HOME="${JAVA_HOME:-/opt/jbr-21}"
bash "$ROOT/scripts/build.sh" "$MODE"

case "$MODE" in
  release) APK="$ROOT/build/app/outputs/apk/release/app-release.apk" ;;
  debug)   APK="$ROOT/build/app/outputs/apk/debug/app-debug.apk" ;;
esac

adb install -r "$APK"
adb shell am start -W -n io.bitshift.zipp/.MainActivity
exec adb logcat -s zipp
```

**Step 3: `app/scripts/release.sh`**

Adapt `pige/scripts/release.sh` with `pige` -> `zipp`, artefact names
`zipp-$VERSION-linux-$(uname -m)` and `zipp-$VERSION-android-arm64.apk`, and the
same `all|android|desktop` modes. Keep the rule that cleans stale versions from
`dist/`.

**Step 4: `app/scripts/publish.sh`**

Adapt `pige/scripts/publish.sh` with `pige` -> `zipp`, tag `v$VERSION`, title
`Zipp $VERSION`, and the same `--draft`, `gh auth`, tag-exists and artefact
checks.

**Step 5: Rewrite `app/README.md`**

Cover:

- What Zipp is and the two screens.
- Requirements: Rust, JDK 21, Android SDK 36, NDK `28.2.13676358`, Gradle
  wrapper, Linux BlueZ for desktop BLE.
- Commands: `./run.sh`, `cargo test --workspace`, `bash scripts/build.sh debug`,
  `./run_adb.sh`, `bash scripts/release.sh`,
  `bash scripts/publish.sh`.
- The Entry URL and the cleartext-HTTP requirement.
- The logcat tag `zipp`.
- A link to `ARCHITECTURE.md` and `docs/technical-notes.md`.

**Step 6: Rewrite the root `README.md`**

Change `app-java` to `app` and describe the iced app:

```markdown
* mcu-nrf-rs - mcu software
* app - iced dashboard (Linux + Android)
* docs - documentation
* diagrams - circuit diagrams
* old is archived on the `old` branch
```

**Step 7: Add the root `.gitignore`**

The repository has none. Add:

```gitignore
target/
build/
dist/
.gradle/
.kotlin/
.idea/
*.iml
*.apk
*.aab
local.properties
keystore.properties
```

**Step 8: Fill the technical notes**

Run the measurements and record them. Use `du -h` for artefact sizes and
`adb logcat` for the device result.

**Step 9: Check and commit**

```sh
chmod +x app/run.sh app/run_adb.sh app/scripts/*.sh
bash -n app/run.sh app/run_adb.sh app/scripts/*.sh
bash app/scripts/version.sh
git add -A
git commit -m "docs: rewrite build, run and release instructions"
```

---

## Task 21: Final verification and release build

**Files**

- Fix any files the final gate exposes

**Step 1: Run the full gate**

```sh
cd /home/bronson/Projects/zipp/app/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p zipp --target aarch64-linux-android
```

**Step 2: Build both artefacts**

```sh
cd /home/bronson/Projects/zipp/app
bash scripts/release.sh all
ls -l dist/
```

Expected:

```text
dist/zipp-2026.11.8-linux-x86_64
dist/zipp-2026.11.8-android-arm64.apk
```

The Linux artefact name uses `uname -m`; on this machine it is `x86_64`.

**Step 3: Run the Flutter-free gate**

```sh
cd /home/bronson/Projects/zipp
grep -RinE 'flutter|dart|pubspec|flutter_rust_bridge' \
  --exclude-dir=old --exclude-dir=.git --exclude-dir=target \
  app
```

Only the design/plan prose in `app/docs/superpowers/` may match.

**Step 4: Manual acceptance**

Desktop:

- [ ] `./run.sh` opens the dashboard.
- [ ] The layout matches the Flutter screenshots: 480 dp dashboard, five rows,
      bottom scanner.
- [ ] Entry sends a GET and logs the result.
- [ ] The scanner never crashes when Bluetooth is missing or off.
- [ ] With a scooter, scan/connect/notify/read/write work.

Android:

- [ ] `./run_adb.sh release` installs and launches.
- [ ] Permissions, scan, auto-connect, dashboard values, every button, log page,
      UART TX/RX and GPS all work.
- [ ] Rotate, background/foreground and Back behave.
- [ ] `adb logcat -s zipp` has no panic, JNI exception or ANR.

**Step 5: Commit any fixes and tag**

```sh
cd /home/bronson/Projects/zipp
git add -A
git commit -m "chore: final verification fixes"   # only if there are changes
git status --short --branch
```

Do not create a tag or push unless the user asks. The plan is complete when the
two artefacts exist and the manual checklists pass.

---

## Self-review notes

- **Spec coverage:** workspace, protocol, transport, session, i18n, GPS, Entry,
  log, dashboard, scanner, log page, desktop BLE, Android BLE/GPS/Wi-Fi,
  JNI, Gradle shell, docs, scripts, Flutter removal and verification all have a
  task.
- **Deliberate deviations:** the page switch is instant (recorded in the
  technical notes); the baseline alignment in the speed row uses bottom
  alignment because iced has no text-baseline alignment. Both are minor and
  recorded.
- **Type consistency:** `Transport::send` is synchronous; all blocking/async
  work lives in the platform actor or a `Task`. `ScooterSession` is pure and
  takes `NaiveTime` so tests are deterministic. `DeviceId` is the only device
  key across the UI, session and transport.
- **No hidden global state:** the desktop/Android transport is a `OnceLock`
  installed once; Android's JNI handle is the only other process-wide state.
- **Testability:** all protocol, session, i18n, GPS, Entry, log and UI helper
  logic is testable without iced or hardware. The only hardware-dependent
  checks are in Tasks 12, 16, 17 and 21.

