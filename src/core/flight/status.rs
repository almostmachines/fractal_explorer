#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightWarning {
    SpeedClamped,
    CenterClamped,
    ExtentClamped,
    NonFiniteReset,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlightStatus {
    pub paused: bool,
    pub speed: f64,
    pub heading: [f64; 2],
    pub last_warning: Option<FlightWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlightUpdateReport {
    pub clamped: bool,
    pub warning: Option<FlightWarning>,
}

impl Default for FlightStatus {
    fn default() -> Self {
        Self {
            paused: true,
            speed: 0.0,
            heading: [0.0, 0.0],
            last_warning: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FlightStatus, FlightWarning};

    #[test]
    fn default_status_starts_paused() {
        let status = FlightStatus::default();

        assert!(status.paused);
        assert_eq!(status.speed, 0.0);
        assert_eq!(status.heading, [0.0, 0.0]);
        assert_eq!(status.last_warning, None);
    }

    #[test]
    fn warning_can_be_attached_to_status() {
        let status = FlightStatus {
            last_warning: Some(FlightWarning::SpeedClamped),
            ..FlightStatus::default()
        };

        assert_eq!(status.last_warning, Some(FlightWarning::SpeedClamped));
    }
}
