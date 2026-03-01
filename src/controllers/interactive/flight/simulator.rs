use crate::core::flight::{
    FlightControlsSnapshot, FlightLimits, FlightStatus, FlightUpdateReport, MotionState,
    step_motion,
};
use std::time::Duration;

pub struct FlightSimulator {
    motion: MotionState,
    limits: FlightLimits,
    accumulator_secs: f64,
    status: FlightStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    pub state_changed: bool,
    pub ticks_run: u32,
    pub status: FlightStatus,
}

impl FlightSimulator {
    #[must_use]
    pub fn new(limits: FlightLimits) -> Self {
        Self {
            motion: MotionState::default(),
            limits,
            accumulator_secs: 0.0,
            status: FlightStatus::default(),
        }
    }

    pub fn advance<C, U>(
        &mut self,
        elapsed: Duration,
        mut controls_fn: C,
        mut update_fractal: U,
    ) -> SimulationResult
    where
        C: FnMut() -> FlightControlsSnapshot,
        U: FnMut(&MotionState, f64, &FlightLimits) -> FlightUpdateReport,
    {
        let dt = self.limits.dt();
        if !dt.is_finite() || dt <= 0.0 {
            return SimulationResult {
                state_changed: false,
                ticks_run: 0,
                status: self.status.clone(),
            };
        }

        self.accumulator_secs += elapsed.as_secs_f64();
        if !self.accumulator_secs.is_finite() || self.accumulator_secs < 0.0 {
            self.accumulator_secs = 0.0;
        }

        let ticks_available = (self.accumulator_secs / dt).floor();
        let max_ticks = f64::from(self.limits.max_ticks_per_redraw);
        let ticks_run = ticks_available.min(max_ticks) as u32;
        let dropped_excess = ticks_available > max_ticks;

        let mut state_changed = false;

        for _ in 0..ticks_run {
            let controls = controls_fn();
            let previous_motion = self.motion;
            let previous_status = self.status.clone();

            let motion_report = step_motion(&mut self.motion, controls, dt, &self.limits);
            let update_report = update_fractal(&self.motion, dt, &self.limits);

            self.status.paused = self.motion.paused;
            self.status.speed = self.motion.speed_world_per_sec;
            self.status.heading = self.motion.heading;
            self.status.last_warning = update_report.warning.or(motion_report.warning);

            if previous_motion != self.motion
                || self.status != previous_status
                || motion_report.view_should_update
                || update_report.clamped
            {
                state_changed = true;
            }
        }

        if dropped_excess {
            self.accumulator_secs = 0.0;
        } else {
            self.accumulator_secs -= f64::from(ticks_run) * dt;
            if self.accumulator_secs < 0.0 {
                self.accumulator_secs = 0.0;
            }
        }

        SimulationResult {
            state_changed,
            ticks_run,
            status: self.status.clone(),
        }
    }

    pub fn reset_motion(&mut self) {
        self.motion = MotionState::default();
        self.status = FlightStatus::default();
        self.accumulator_secs = 0.0;
    }

    #[must_use]
    pub fn status(&self) -> &FlightStatus {
        &self.status
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.motion.paused
            && (self.motion.speed_world_per_sec != 0.0 || self.motion.accel_world_per_sec2 != 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::FlightSimulator;
    use crate::core::flight::{
        FlightControlsSnapshot, FlightLimits, FlightUpdateReport, FlightWarning, MotionState,
    };
    use std::time::Duration;

    fn test_limits() -> FlightLimits {
        FlightLimits {
            tick_hz: 60,
            max_ticks_per_redraw: 10,
            ..FlightLimits::default()
        }
    }

    #[test]
    fn exact_tick_runs_one_step() {
        let mut simulator = FlightSimulator::new(test_limits());
        let mut controls_calls = 0;
        let mut fractal_calls = 0;

        let result = simulator.advance(
            Duration::from_secs_f64(1.0 / 60.0),
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| {
                fractal_calls += 1;
                FlightUpdateReport::default()
            },
        );

        assert_eq!(result.ticks_run, 1);
        assert_eq!(controls_calls, 1);
        assert_eq!(fractal_calls, 1);
    }

    #[test]
    fn multiple_ticks_run_for_larger_elapsed() {
        let mut simulator = FlightSimulator::new(test_limits());
        let mut controls_calls = 0;
        let mut fractal_calls = 0;

        let result = simulator.advance(
            Duration::from_secs_f64(3.0 / 60.0),
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| {
                fractal_calls += 1;
                FlightUpdateReport::default()
            },
        );

        assert_eq!(result.ticks_run, 3);
        assert_eq!(controls_calls, 3);
        assert_eq!(fractal_calls, 3);
    }

    #[test]
    fn fractional_elapsed_rolls_over_to_next_advance() {
        let mut limits = test_limits();
        limits.tick_hz = 2;

        let mut simulator = FlightSimulator::new(limits);
        let mut controls_calls = 0;

        let first = simulator.advance(
            Duration::from_secs_f64(0.25),
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        let second = simulator.advance(
            Duration::from_secs_f64(0.25),
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        assert_eq!(first.ticks_run, 0);
        assert_eq!(second.ticks_run, 1);
        assert_eq!(controls_calls, 1);
    }

    #[test]
    fn zero_elapsed_runs_no_ticks() {
        let mut simulator = FlightSimulator::new(test_limits());
        let mut controls_calls = 0;

        let result = simulator.advance(
            Duration::ZERO,
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        assert_eq!(result.ticks_run, 0);
        assert_eq!(controls_calls, 0);
        assert!(!result.state_changed);
    }

    #[test]
    fn large_elapsed_is_capped_and_excess_time_is_dropped() {
        let mut simulator = FlightSimulator::new(test_limits());
        let mut controls_calls = 0;

        let capped = simulator.advance(
            Duration::from_secs(1),
            || {
                controls_calls += 1;
                FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        assert_eq!(capped.ticks_run, 10);
        assert_eq!(controls_calls, 10);

        let after_drop = simulator.advance(
            Duration::ZERO,
            FlightControlsSnapshot::default,
            |_, _, _| FlightUpdateReport::default(),
        );

        assert_eq!(after_drop.ticks_run, 0);
    }

    #[test]
    fn paused_state_reports_no_state_change() {
        let mut simulator = FlightSimulator::new(test_limits());

        let _ = simulator.advance(
            Duration::from_secs_f64(1.0 / 60.0),
            || FlightControlsSnapshot {
                pause_toggle_edge: true,
                ..FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        let result = simulator.advance(
            Duration::from_secs_f64(1.0 / 60.0),
            || FlightControlsSnapshot {
                accelerate: true,
                ..FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport::default(),
        );

        assert_eq!(result.ticks_run, 1);
        assert!(!result.state_changed);
        assert!(result.status.paused);
    }

    #[test]
    fn reset_motion_restores_defaults() {
        let mut simulator = FlightSimulator::new(test_limits());

        let _ = simulator.advance(
            Duration::from_secs_f64(1.0 / 60.0),
            || FlightControlsSnapshot {
                accelerate: true,
                ..FlightControlsSnapshot::default()
            },
            |_, _, _| FlightUpdateReport {
                clamped: true,
                warning: Some(FlightWarning::CenterClamped),
            },
        );

        simulator.reset_motion();

        assert_eq!(simulator.status().paused, false);
        assert_eq!(simulator.status().speed, 0.0);
        assert_eq!(simulator.status().heading, [0.0, -1.0]);
        assert_eq!(simulator.status().last_warning, None);
        assert!(!simulator.is_active());
    }

    #[test]
    fn is_active_true_when_speed_nonzero() {
        let mut simulator = FlightSimulator::new(test_limits());
        simulator.motion.speed_world_per_sec = 1.0;

        assert!(simulator.is_active());
    }

    #[test]
    fn is_active_false_when_paused_even_with_speed() {
        let mut simulator = FlightSimulator::new(test_limits());
        simulator.motion.paused = true;
        simulator.motion.speed_world_per_sec = 1.0;

        assert!(!simulator.is_active());
    }

    #[test]
    fn is_active_false_when_stationary() {
        let simulator = FlightSimulator::new(test_limits());

        assert!(!simulator.is_active());
    }

    #[test]
    fn status_reflects_motion_and_warnings() {
        let mut simulator = FlightSimulator::new(test_limits());

        let result = simulator.advance(
            Duration::from_secs_f64(1.0 / 60.0),
            || FlightControlsSnapshot {
                accelerate: true,
                ..FlightControlsSnapshot::default()
            },
            |motion: &MotionState, _, _| {
                if motion.speed_world_per_sec > 0.0 {
                    FlightUpdateReport {
                        clamped: true,
                        warning: Some(FlightWarning::ExtentClamped),
                    }
                } else {
                    FlightUpdateReport::default()
                }
            },
        );

        assert_eq!(result.status.paused, simulator.status().paused);
        assert_eq!(result.status.speed, simulator.status().speed);
        assert_eq!(result.status.heading, simulator.status().heading);
        assert_eq!(
            result.status.last_warning,
            Some(FlightWarning::ExtentClamped)
        );
        assert!(result.state_changed);
    }
}
