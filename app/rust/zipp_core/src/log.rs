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
        assert_eq!(log.lines().iter().next().unwrap(), "09:05 start scan");
    }

    #[test]
    fn empty_messages_keep_the_timestamp_and_a_space() {
        let mut log = LogBuffer::new();
        log.push("", at(23, 59));
        assert_eq!(log.lines().iter().next().unwrap(), "23:59 ");
    }

    #[test]
    fn keeps_only_the_newest_two_hundred_lines() {
        let mut log = LogBuffer::new();
        for index in 0..205 {
            log.push(index.to_string(), at(1, 0));
        }
        assert_eq!(log.len(), MAX_LOGS);
        assert_eq!(log.lines().iter().next().unwrap(), "01:00 5");
    }

    #[test]
    fn clear_empties_the_buffer() {
        let mut log = LogBuffer::new();
        log.push("x", at(1, 1));
        log.clear();
        assert!(log.is_empty());
    }
}
