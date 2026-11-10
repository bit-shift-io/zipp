//! User-facing labels in English and Chinese.

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
            self.power,
            self.alarm,
            self.lights,
            self.sport,
            self.cruise,
            self.park,
            self.brake,
            self.horn,
            self.entry,
            self.cruise_up,
            self.cruise_down,
            self.kmh,
            self.km,
            self.celsius,
            self.mv,
            self.watts,
            self.on,
            self.off,
            self.scanner_title,
            self.scanning,
            self.connecting,
            self.disconnected,
            self.connect,
            self.disconnect,
            self.unknown,
            self.nearby,
            self.log,
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
        if let Ok(value) = std::env::var(key)
            && !value.is_empty()
        {
            return Lang::from_tag(&value);
        }
    }
    Lang::En
}

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
    fn strings_match_the_legacy_values() {
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
