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

pub fn parse_characteristic_data(state: ScooterState, uuid: &str, data: &[u8]) -> ParseResult {
    let mut new_state = state;
    let mut log = None;

    // Normalize UUID (extract 16-bit part if it matches base)
    let uuid_16 = if uuid.len() == 36 {
        let lower = uuid.to_lowercase();
        if lower.starts_with("0000") && lower.ends_with("-0000-1000-8000-00805f9b34fb") {
            lower[4..8].to_string()
        } else {
            lower // Keep full if custom (like likely UART)
        }
    } else {
        uuid.to_lowercase()
    };

    match uuid_16.as_str() {
        // 1000 series is settings
        "1001" => {
            if !data.is_empty() {
                new_state.power_on = data[0] != 0;
            }
        }
        "1003" => {
            if !data.is_empty() {
                new_state.lights_on = data[0] != 0;
            }
        }
        "1004" => {
            if !data.is_empty() {
                new_state.alarm_on = data[0] != 0;
            }
        }
        "1005" => {
            if !data.is_empty() {
                new_state.sport_on = data[0] != 0;
            }
        }

        // 2000 series is data
        "2001" => {
            if !data.is_empty() {
                new_state.speed = format!("{:02}", data[0]);
            }
        }
        "2002" => {
            if data.len() == 2 {
                let v = (data[0] as u16) | ((data[1] as u16) << 8);
                new_state.throttle_level = v.to_string();
            }
        }
        "2004" => {
            if !data.is_empty() {
                new_state.temperature = format!("{:02}", data[0]);
            }
        }
        "2005" => {
            if !data.is_empty() {
                new_state.clock_minutes = format!("{:02}", data[0]);
            }
        }
        "2006" => {
            if !data.is_empty() {
                new_state.clock_hours = format!("{:02}", data[0]);
            }
        }
        "2007" => {
            if !data.is_empty() {
                new_state.brake_active = data[0] != 0;
            }
        }
        "2008" => {
            if !data.is_empty() {
                new_state.park_brake_active = data[0] != 0;
            }
        }
        "2009" => {
            if !data.is_empty() {
                new_state.cruise_level = data[0] as i32;
            }
        }

        // Battery
        "2a19" => {
            if !data.is_empty() {
                new_state.battery_level = data[0].to_string();
            }
        }
        "2b05" => {
            if data.len() == 2 {
                let v = (data[0] as u16) | ((data[1] as u16) << 8);
                new_state.battery_power = v.to_string();
            }
        }
        "2003" => {
            if data.len() >= 2 {
                let v = (data[0] as u16) | ((data[1] as u16) << 8);
                new_state.odometer = format!("{:04}", v);
            }
        }
        // UART RX
        "0003" | "6e400003-b5a3-f393-e0a9-e50e24dcca9e" => {
            if let Ok(s) = String::from_utf8(data.to_vec()) {
                log = Some(s);
            }
        }
        _ => {}
    }

    ParseResult {
        state: new_state,
        log,
    }
}

pub fn create_command_bytes(command: &ScooterCommand, current_state: &ScooterState) -> Vec<u8> {
    match command {
        ScooterCommand::TogglePower => {
            if current_state.power_on {
                vec![0]
            } else {
                vec![183]
            }
        }
        ScooterCommand::ToggleAlarm => {
            if current_state.alarm_on {
                vec![0]
            } else {
                vec![205]
            }
        }
        ScooterCommand::ToggleSport => {
            if current_state.sport_on {
                vec![0]
            } else {
                vec![1]
            }
        }
        ScooterCommand::ToggleLights => {
            if current_state.lights_on {
                vec![0]
            } else {
                vec![1]
            }
        }
        ScooterCommand::CruiseUp => vec![1],
        ScooterCommand::CruiseDown => vec![0],
        ScooterCommand::Horn => vec![1],
        ScooterCommand::SetUart(s) => s.clone().into_bytes(),
    }
}

pub fn get_command_action(command: &ScooterCommand, current_state: &ScooterState) -> BleAction {
    let bytes = create_command_bytes(command, current_state);

    let (service, characteristic) = match command {
        ScooterCommand::TogglePower => ("1000", "1001"),
        ScooterCommand::ToggleLights => ("1000", "1003"),
        ScooterCommand::ToggleAlarm => ("1000", "1004"),
        ScooterCommand::ToggleSport => ("1000", "1005"),
        ScooterCommand::CruiseUp | ScooterCommand::CruiseDown => ("1000", "1006"),
        ScooterCommand::Horn => ("1000", "1002"),
        ScooterCommand::SetUart(_) => (
            "6e400001-b5a3-f393-e0a9-e50e24dcca9e",
            "6e400002-b5a3-f393-e0a9-e50e24dcca9e",
        ),
    };

    let expand_16 = |s: &str| -> String {
        if s.len() == 4 {
            format!("0000{}-0000-1000-8000-00805F9B34FB", s)
        } else {
            s.to_string()
        }
    };

    BleAction {
        bytes,
        service_uuid: expand_16(service),
        characteristic_uuid: expand_16(characteristic),
    }
}

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
    fn data_characteristics_match_the_legacy_mapping() {
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
        let upper =
            parse_characteristic_data(state(), "00002001-0000-1000-8000-00805F9B34FB", &[9]);
        let lower =
            parse_characteristic_data(state(), "00002001-0000-1000-8000-00805f9b34fb", &[9]);
        assert_eq!(upper.state.speed, "09");
        assert_eq!(lower.state.speed, "09");
    }

    #[test]
    fn command_bytes_are_exact() {
        let mut s = state();
        assert_eq!(
            create_command_bytes(&ScooterCommand::TogglePower, &s),
            vec![183]
        );
        s.power_on = true;
        assert_eq!(
            create_command_bytes(&ScooterCommand::TogglePower, &s),
            vec![0]
        );

        let mut s = state();
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleAlarm, &s),
            vec![205]
        );
        s.alarm_on = true;
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleAlarm, &s),
            vec![0]
        );

        let mut s = state();
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleSport, &s),
            vec![1]
        );
        s.sport_on = true;
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleSport, &s),
            vec![0]
        );

        let mut s = state();
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleLights, &s),
            vec![1]
        );
        s.lights_on = true;
        assert_eq!(
            create_command_bytes(&ScooterCommand::ToggleLights, &s),
            vec![0]
        );

        assert_eq!(create_command_bytes(&ScooterCommand::CruiseUp, &s), vec![1]);
        assert_eq!(
            create_command_bytes(&ScooterCommand::CruiseDown, &s),
            vec![0]
        );
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
            (
                ScooterCommand::TogglePower,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001001-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::ToggleLights,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001003-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::ToggleAlarm,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001004-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::ToggleSport,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001005-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::CruiseUp,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001006-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::CruiseDown,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001006-0000-1000-8000-00805F9B34FB",
            ),
            (
                ScooterCommand::Horn,
                "00001000-0000-1000-8000-00805F9B34FB",
                "00001002-0000-1000-8000-00805F9B34FB",
            ),
        ];
        for (command, service, characteristic) in cases {
            let action = get_command_action(&command, &s);
            assert_eq!(action.service_uuid, service, "{command}");
            assert_eq!(action.characteristic_uuid, characteristic, "{command}");
        }
        let uart = get_command_action(&ScooterCommand::SetUart("x".into()), &s);
        assert_eq!(uart.service_uuid, "6e400001-b5a3-f393-e0a9-e50e24dcca9e");
        assert_eq!(
            uart.characteristic_uuid,
            "6e400002-b5a3-f393-e0a9-e50e24dcca9e"
        );
    }

    #[test]
    fn power_action_expands_service_and_characteristic() {
        let action = get_command_action(&ScooterCommand::TogglePower, &state());
        assert_eq!(action.service_uuid, "00001000-0000-1000-8000-00805F9B34FB");
        assert_eq!(
            action.characteristic_uuid,
            "00001001-0000-1000-8000-00805F9B34FB"
        );
    }
}
