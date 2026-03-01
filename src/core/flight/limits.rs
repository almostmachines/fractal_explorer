#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightLimits {
    pub tick_hz: u32,
    pub base_accel_world_per_sec2: f64,
    pub max_speed_abs_world_per_sec: f64,
    pub min_region_extent: f64,
    pub max_region_extent: f64,
    pub max_center_abs: f64,
    pub zoom_base: f64,
    pub steer_strength: f64,
    pub max_ticks_per_redraw: u32,
}

impl FlightLimits {
    #[must_use]
    pub fn dt(&self) -> f64 {
        if self.tick_hz == 0 {
            0.0
        } else {
            1.0 / f64::from(self.tick_hz)
        }
    }
}

impl Default for FlightLimits {
    fn default() -> Self {
        Self {
            tick_hz: 60,
            base_accel_world_per_sec2: 0.5,
            max_speed_abs_world_per_sec: 5.0,
            min_region_extent: 1e-15,
            max_region_extent: 20.0,
            max_center_abs: 100.0,
            zoom_base: 2.0,
            steer_strength: 0.5,
            max_ticks_per_redraw: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FlightLimits;

    #[test]
    fn default_limits_are_finite_and_consistent() {
        let limits = FlightLimits::default();

        assert!(limits.tick_hz > 0);
        assert!(limits.base_accel_world_per_sec2.is_finite());
        assert!(limits.max_speed_abs_world_per_sec.is_finite());
        assert!(limits.min_region_extent.is_finite());
        assert!(limits.max_region_extent.is_finite());
        assert!(limits.max_center_abs.is_finite());
        assert!(limits.zoom_base.is_finite());
        assert!(limits.steer_strength.is_finite());
        assert!(limits.max_ticks_per_redraw > 0);
        assert!(limits.min_region_extent > 0.0);
        assert!(limits.max_region_extent >= limits.min_region_extent);
        assert!(limits.max_speed_abs_world_per_sec >= 0.0);
        assert!(limits.max_center_abs >= 0.0);
        assert!(limits.zoom_base > 0.0);
        assert_ne!(limits.zoom_base, 1.0);
        assert!(limits.steer_strength >= 0.0);
    }

    #[test]
    fn dt_matches_tick_rate() {
        let limits = FlightLimits::default();

        assert!((limits.dt() - (1.0 / 60.0)).abs() < f64::EPSILON);
        assert!(limits.dt().is_finite());
        assert!(limits.dt() > 0.0);
    }
}
