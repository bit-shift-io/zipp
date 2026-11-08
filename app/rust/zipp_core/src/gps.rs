//! GPS status and speed conversion, independent of the platform source.

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
        let update = GpsUpdate {
            status: GpsStatus::Active,
            speed_mps: Some(2.5),
        };
        assert!((update.speed_kmh().unwrap() - 9.0).abs() < 1e-9);
        assert_eq!(
            GpsUpdate {
                status: GpsStatus::Searching,
                speed_mps: None
            }
            .speed_kmh(),
            None
        );
    }
}
