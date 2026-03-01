use crate::core::flight::controls::FlightControlsSnapshot;
use crate::core::flight::limits::FlightLimits;
use crate::core::flight::status::FlightWarning;

const DEFAULT_HEADING: [f64; 2] = [0.0, -1.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionState {
    pub paused: bool,
    pub heading: [f64; 2],
    pub speed_world_per_sec: f64,
    pub accel_world_per_sec2: f64,
}

impl Default for MotionState {
    fn default() -> Self {
        Self {
            paused: false,
            heading: DEFAULT_HEADING,
            speed_world_per_sec: 0.0,
            accel_world_per_sec2: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MotionStepReport {
    pub pause_toggled: bool,
    pub speed_clamped: bool,
    pub view_should_update: bool,
    pub warning: Option<FlightWarning>,
}

pub fn step_motion(
    motion: &mut MotionState,
    controls: FlightControlsSnapshot,
    dt: f64,
    limits: &FlightLimits,
) -> MotionStepReport {
    let mut report = MotionStepReport::default();

    if controls.pause_toggle_edge {
        motion.paused = !motion.paused;
        report.pause_toggled = true;
    }

    if motion.paused {
        motion.accel_world_per_sec2 = 0.0;
        return report;
    }

    resolve_heading(motion, controls);

    motion.accel_world_per_sec2 =
        effective_acceleration(controls, limits.base_accel_world_per_sec2);

    let safe_dt = if dt.is_finite() { dt } else { 0.0 };
    motion.speed_world_per_sec += motion.accel_world_per_sec2 * safe_dt;

    let max_speed_abs = limits.max_speed_abs_world_per_sec.abs();
    if motion.speed_world_per_sec > max_speed_abs {
        motion.speed_world_per_sec = max_speed_abs;
        report.speed_clamped = true;
        report.warning = Some(FlightWarning::SpeedClamped);
    } else if motion.speed_world_per_sec < -max_speed_abs {
        motion.speed_world_per_sec = -max_speed_abs;
        report.speed_clamped = true;
        report.warning = Some(FlightWarning::SpeedClamped);
    }

    report.view_should_update = motion.speed_world_per_sec != 0.0;
    report
}

fn resolve_heading(motion: &mut MotionState, controls: FlightControlsSnapshot) {
    let x = axis_from_pair(controls.d, controls.a);
    let y = axis_from_pair(controls.s, controls.w);
    let length_sq = (x * x) + (y * y);

    if length_sq > 0.0 {
        let inv_length = length_sq.sqrt().recip();
        motion.heading = [x * inv_length, y * inv_length];
    }
}

fn axis_from_pair(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn effective_acceleration(controls: FlightControlsSnapshot, base_accel: f64) -> f64 {
    let mut accel = 0.0;
    if controls.accelerate {
        accel += base_accel;
    }
    if controls.decelerate {
        accel -= base_accel;
    }
    accel
}

#[cfg(test)]
mod tests {
    use super::{MotionState, step_motion};
    use crate::core::flight::{FlightControlsSnapshot, FlightLimits, FlightWarning};
    use std::f64::consts::FRAC_1_SQRT_2;

    const EPSILON: f64 = 1e-12;

    fn assert_approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "actual={} expected={}",
            actual,
            expected
        );
    }

    fn default_limits() -> FlightLimits {
        FlightLimits::default()
    }

    #[test]
    fn default_motion_state_matches_spec_defaults() {
        let motion = MotionState::default();

        assert!(!motion.paused);
        assert_eq!(motion.heading, [0.0, -1.0]);
        assert_eq!(motion.speed_world_per_sec, 0.0);
        assert_eq!(motion.accel_world_per_sec2, 0.0);
    }

    #[test]
    fn w_only_sets_up_heading() {
        let mut motion = MotionState {
            heading: [1.0, 0.0],
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            w: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [0.0, -1.0]);
    }

    #[test]
    fn s_only_sets_down_heading() {
        let mut motion = MotionState {
            heading: [1.0, 0.0],
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            s: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [0.0, 1.0]);
    }

    #[test]
    fn a_only_sets_left_heading() {
        let mut motion = MotionState {
            heading: [0.0, -1.0],
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            a: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [-1.0, 0.0]);
    }

    #[test]
    fn d_only_sets_right_heading() {
        let mut motion = MotionState {
            heading: [0.0, -1.0],
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            d: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [1.0, 0.0]);
    }

    #[test]
    fn w_and_d_normalizes_diagonal_heading() {
        let mut motion = MotionState::default();
        let controls = FlightControlsSnapshot {
            w: true,
            d: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_approx_eq(motion.heading[0], FRAC_1_SQRT_2);
        assert_approx_eq(motion.heading[1], -FRAC_1_SQRT_2);

        let heading_length =
            (motion.heading[0] * motion.heading[0] + motion.heading[1] * motion.heading[1]).sqrt();
        assert_approx_eq(heading_length, 1.0);
    }

    #[test]
    fn no_wasd_keeps_existing_heading() {
        let mut motion = MotionState {
            heading: [1.0, 0.0],
            ..MotionState::default()
        };

        step_motion(
            &mut motion,
            FlightControlsSnapshot::default(),
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [1.0, 0.0]);
    }

    #[test]
    fn all_wasd_cancels_to_zero_vector_and_keeps_heading() {
        let mut motion = MotionState {
            heading: [0.0, 1.0],
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            w: true,
            a: true,
            s: true,
            d: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.heading, [0.0, 1.0]);
    }

    #[test]
    fn accelerate_increases_speed_by_accel_times_dt() {
        let mut motion = MotionState {
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let limits = default_limits();
        let controls = FlightControlsSnapshot {
            accelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 0.5, &limits);

        assert_approx_eq(
            motion.accel_world_per_sec2,
            limits.base_accel_world_per_sec2,
        );
        assert_approx_eq(motion.speed_world_per_sec, 1.25);
    }

    #[test]
    fn decelerate_decreases_speed_by_accel_times_dt() {
        let mut motion = MotionState {
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let limits = default_limits();
        let controls = FlightControlsSnapshot {
            decelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 0.5, &limits);

        assert_approx_eq(
            motion.accel_world_per_sec2,
            -limits.base_accel_world_per_sec2,
        );
        assert_approx_eq(motion.speed_world_per_sec, 0.75);
    }

    #[test]
    fn accelerate_and_decelerate_cancel_out() {
        let mut motion = MotionState {
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            accelerate: true,
            decelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 1.0, &default_limits());

        assert_eq!(motion.accel_world_per_sec2, 0.0);
        assert_eq!(motion.speed_world_per_sec, 1.0);
    }

    #[test]
    fn no_acceleration_input_sets_accel_to_zero() {
        let mut motion = MotionState {
            accel_world_per_sec2: 123.0,
            ..MotionState::default()
        };

        step_motion(
            &mut motion,
            FlightControlsSnapshot::default(),
            default_limits().dt(),
            &default_limits(),
        );

        assert_eq!(motion.accel_world_per_sec2, 0.0);
    }

    #[test]
    fn deceleration_can_reverse_speed_through_zero() {
        let mut motion = MotionState {
            speed_world_per_sec: 0.1,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            decelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 0.5, &default_limits());

        assert_approx_eq(motion.speed_world_per_sec, -0.15);
    }

    #[test]
    fn positive_speed_is_clamped_to_max() {
        let limits = default_limits();
        let mut motion = MotionState {
            speed_world_per_sec: limits.max_speed_abs_world_per_sec - 0.01,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            accelerate: true,
            ..FlightControlsSnapshot::default()
        };

        let report = step_motion(&mut motion, controls, 1.0, &limits);

        assert_eq!(
            motion.speed_world_per_sec,
            limits.max_speed_abs_world_per_sec
        );
        assert!(report.speed_clamped);
        assert_eq!(report.warning, Some(FlightWarning::SpeedClamped));
    }

    #[test]
    fn negative_speed_is_clamped_to_min() {
        let limits = default_limits();
        let mut motion = MotionState {
            speed_world_per_sec: -limits.max_speed_abs_world_per_sec + 0.01,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            decelerate: true,
            ..FlightControlsSnapshot::default()
        };

        let report = step_motion(&mut motion, controls, 1.0, &limits);

        assert_eq!(
            motion.speed_world_per_sec,
            -limits.max_speed_abs_world_per_sec
        );
        assert!(report.speed_clamped);
        assert_eq!(report.warning, Some(FlightWarning::SpeedClamped));
    }

    #[test]
    fn pause_edge_toggles_paused_state() {
        let mut motion = MotionState::default();
        let controls = FlightControlsSnapshot {
            pause_toggle_edge: true,
            ..FlightControlsSnapshot::default()
        };

        let report = step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert!(motion.paused);
        assert!(report.pause_toggled);
        assert!(!report.view_should_update);
    }

    #[test]
    fn pause_edge_while_paused_unpauses() {
        let mut motion = MotionState {
            paused: true,
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            pause_toggle_edge: true,
            ..FlightControlsSnapshot::default()
        };

        let report = step_motion(
            &mut motion,
            controls,
            default_limits().dt(),
            &default_limits(),
        );

        assert!(!motion.paused);
        assert_eq!(motion.speed_world_per_sec, 1.0);
        assert!(report.pause_toggled);
        assert!(report.view_should_update);
    }

    #[test]
    fn paused_motion_does_not_advance_heading_or_speed() {
        let mut motion = MotionState {
            paused: true,
            heading: [1.0, 0.0],
            speed_world_per_sec: 1.0,
            accel_world_per_sec2: 2.0,
        };
        let controls = FlightControlsSnapshot {
            w: true,
            accelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 1.0, &default_limits());

        assert_eq!(motion.heading, [1.0, 0.0]);
        assert_eq!(motion.speed_world_per_sec, 1.0);
        assert_eq!(motion.accel_world_per_sec2, 0.0);
    }

    #[test]
    fn pause_toggle_edge_affects_only_one_tick_when_consumed() {
        let mut motion = MotionState::default();
        let first_controls = FlightControlsSnapshot {
            pause_toggle_edge: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(
            &mut motion,
            first_controls,
            default_limits().dt(),
            &default_limits(),
        );
        assert!(motion.paused);

        step_motion(
            &mut motion,
            FlightControlsSnapshot::default(),
            default_limits().dt(),
            &default_limits(),
        );
        assert!(motion.paused);
    }

    #[test]
    fn zero_dt_does_not_change_speed() {
        let mut motion = MotionState {
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            accelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, 0.0, &default_limits());

        assert_eq!(motion.speed_world_per_sec, 1.0);
    }

    #[test]
    fn non_finite_dt_is_treated_as_zero() {
        let mut motion = MotionState {
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };
        let controls = FlightControlsSnapshot {
            accelerate: true,
            ..FlightControlsSnapshot::default()
        };

        step_motion(&mut motion, controls, f64::NAN, &default_limits());

        assert_eq!(motion.speed_world_per_sec, 1.0);
    }

    #[test]
    fn report_view_should_update_is_false_when_speed_is_zero() {
        let mut motion = MotionState::default();

        let report = step_motion(
            &mut motion,
            FlightControlsSnapshot::default(),
            default_limits().dt(),
            &default_limits(),
        );

        assert!(!report.view_should_update);
    }

    #[test]
    fn report_view_should_update_is_true_when_speed_is_nonzero() {
        let mut motion = MotionState {
            speed_world_per_sec: 0.1,
            ..MotionState::default()
        };

        let report = step_motion(
            &mut motion,
            FlightControlsSnapshot::default(),
            default_limits().dt(),
            &default_limits(),
        );

        assert!(report.view_should_update);
    }
}
