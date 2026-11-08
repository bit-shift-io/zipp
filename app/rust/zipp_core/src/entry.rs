//! HTTP entry policy: when a session should POST to the entry endpoint.

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
