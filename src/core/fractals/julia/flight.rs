use crate::core::{
    data::{complex::Complex, complex_rect::ComplexRect},
    flight::{FlightLimits, FlightUpdateReport, FlightWarning, MotionState},
    fractals::julia::julia_config::{JuliaConfig, default_region},
};

pub fn step_flight(
    config: &mut JuliaConfig,
    motion: &MotionState,
    dt: f64,
    limits: &FlightLimits,
) -> FlightUpdateReport {
    let mut report = FlightUpdateReport::default();

    if motion.paused || motion.speed_world_per_sec == 0.0 {
        return report;
    }

    let delta_real = motion.heading[0] * motion.speed_world_per_sec * dt * config.region.width();
    let delta_imag = motion.heading[1] * motion.speed_world_per_sec * dt * config.region.height();

    if let Some(region) = translated_region(&config.region, delta_real, delta_imag) {
        config.region = region;
    } else {
        reset_non_finite(config, &mut report);
        return report;
    }

    let max_center_abs = limits.max_center_abs.abs();
    let width = config.region.width();
    let height = config.region.height();
    let (center_real, center_imag) = region_center(&config.region);
    let clamped_center_real = center_real.clamp(-max_center_abs, max_center_abs);
    let clamped_center_imag = center_imag.clamp(-max_center_abs, max_center_abs);

    if clamped_center_real != center_real || clamped_center_imag != center_imag {
        if let Some(region) =
            rebuild_region(clamped_center_real, clamped_center_imag, width, height)
        {
            config.region = region;
            mark_warning(&mut report, FlightWarning::CenterClamped);
        } else {
            reset_non_finite(config, &mut report);
            return report;
        }
    }

    let min_extent = limits.min_region_extent.min(limits.max_region_extent);
    let max_extent = limits.min_region_extent.max(limits.max_region_extent);

    let mut width = config.region.width();
    let mut height = config.region.height();

    let scale = if width < min_extent || height < min_extent {
        let width_scale = if width < min_extent {
            min_extent / width
        } else {
            1.0
        };
        let height_scale = if height < min_extent {
            min_extent / height
        } else {
            1.0
        };
        width_scale.max(height_scale)
    } else if width > max_extent || height > max_extent {
        let width_scale = if width > max_extent {
            max_extent / width
        } else {
            1.0
        };
        let height_scale = if height > max_extent {
            max_extent / height
        } else {
            1.0
        };
        width_scale.min(height_scale)
    } else {
        1.0
    };

    if scale != 1.0 {
        width *= scale;
        height *= scale;
        let (center_real, center_imag) = region_center(&config.region);

        if let Some(region) = rebuild_region(center_real, center_imag, width, height) {
            config.region = region;
            mark_warning(&mut report, FlightWarning::ExtentClamped);
        } else {
            reset_non_finite(config, &mut report);
            return report;
        }
    }

    if !region_is_finite(&config.region) {
        reset_non_finite(config, &mut report);
    }

    report
}

fn translated_region(
    region: &ComplexRect,
    delta_real: f64,
    delta_imag: f64,
) -> Option<ComplexRect> {
    let top_left = region.top_left();
    let bottom_right = region.bottom_right();

    ComplexRect::new(
        Complex {
            real: top_left.real + delta_real,
            imag: top_left.imag + delta_imag,
        },
        Complex {
            real: bottom_right.real + delta_real,
            imag: bottom_right.imag + delta_imag,
        },
    )
    .ok()
}

fn rebuild_region(
    center_real: f64,
    center_imag: f64,
    width: f64,
    height: f64,
) -> Option<ComplexRect> {
    if !center_real.is_finite()
        || !center_imag.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return None;
    }

    let half_width = width * 0.5;
    let half_height = height * 0.5;

    ComplexRect::new(
        Complex {
            real: center_real - half_width,
            imag: center_imag - half_height,
        },
        Complex {
            real: center_real + half_width,
            imag: center_imag + half_height,
        },
    )
    .ok()
}

fn region_center(region: &ComplexRect) -> (f64, f64) {
    let top_left = region.top_left();
    let bottom_right = region.bottom_right();

    (
        (top_left.real + bottom_right.real) * 0.5,
        (top_left.imag + bottom_right.imag) * 0.5,
    )
}

fn region_is_finite(region: &ComplexRect) -> bool {
    let top_left = region.top_left();
    let bottom_right = region.bottom_right();

    top_left.real.is_finite()
        && top_left.imag.is_finite()
        && bottom_right.real.is_finite()
        && bottom_right.imag.is_finite()
        && region.width().is_finite()
        && region.height().is_finite()
}

fn mark_warning(report: &mut FlightUpdateReport, warning: FlightWarning) {
    report.clamped = true;
    report.warning = Some(warning);
}

fn reset_non_finite(config: &mut JuliaConfig, report: &mut FlightUpdateReport) {
    config.region = default_region();
    mark_warning(report, FlightWarning::NonFiniteReset);
}

#[cfg(test)]
mod tests {
    use super::step_flight;
    use crate::core::{
        data::{complex::Complex, complex_rect::ComplexRect},
        flight::{FlightLimits, FlightWarning, MotionState},
        fractals::julia::julia_config::{JuliaConfig, default_region},
    };
    use std::f64::consts::FRAC_1_SQRT_2;

    const EPSILON: f64 = 1e-12;

    fn rect(
        top_left_real: f64,
        top_left_imag: f64,
        bottom_right_real: f64,
        bottom_right_imag: f64,
    ) -> ComplexRect {
        ComplexRect::new(
            Complex {
                real: top_left_real,
                imag: top_left_imag,
            },
            Complex {
                real: bottom_right_real,
                imag: bottom_right_imag,
            },
        )
        .expect("test region should be valid")
    }

    fn motion(heading: [f64; 2], speed_world_per_sec: f64) -> MotionState {
        MotionState {
            heading,
            speed_world_per_sec,
            ..MotionState::default()
        }
    }

    fn assert_approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "actual={} expected={}",
            actual,
            expected
        );
    }

    #[test]
    fn forward_motion_translates_region_by_view_relative_delta() {
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);

        let report = step_flight(&mut config, &motion, 0.5, &FlightLimits::default());

        assert_approx_eq(config.region.top_left().real, 0.0);
        assert_approx_eq(config.region.bottom_right().real, 4.0);
        assert_approx_eq(config.region.top_left().imag, -1.0);
        assert_approx_eq(config.region.bottom_right().imag, 1.0);
        assert!(!report.clamped);
        assert_eq!(report.warning, None);
    }

    #[test]
    fn negative_speed_translates_in_reverse_direction() {
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], -1.0);

        step_flight(&mut config, &motion, 0.5, &FlightLimits::default());

        assert_approx_eq(config.region.top_left().real, -4.0);
        assert_approx_eq(config.region.bottom_right().real, 0.0);
    }

    #[test]
    fn diagonal_heading_translates_both_axes() {
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([FRAC_1_SQRT_2, -FRAC_1_SQRT_2], 1.0);

        step_flight(&mut config, &motion, 1.0, &FlightLimits::default());

        let expected_delta_real = FRAC_1_SQRT_2 * 4.0;
        let expected_delta_imag = -FRAC_1_SQRT_2 * 2.0;

        assert_approx_eq(config.region.top_left().real, -2.0 + expected_delta_real);
        assert_approx_eq(config.region.bottom_right().real, 2.0 + expected_delta_real);
        assert_approx_eq(config.region.top_left().imag, -1.0 + expected_delta_imag);
        assert_approx_eq(config.region.bottom_right().imag, 1.0 + expected_delta_imag);
    }

    #[test]
    fn paused_or_zero_speed_is_a_noop() {
        let original = rect(-2.0, -1.0, 2.0, 1.0);
        let mut paused_config = JuliaConfig {
            region: original,
            ..JuliaConfig::default()
        };
        let paused_motion = MotionState {
            paused: true,
            speed_world_per_sec: 1.0,
            ..MotionState::default()
        };

        let paused_report = step_flight(
            &mut paused_config,
            &paused_motion,
            1.0,
            &FlightLimits::default(),
        );

        assert_eq!(paused_config.region, original);
        assert!(!paused_report.clamped);
        assert_eq!(paused_report.warning, None);

        let mut zero_speed_config = JuliaConfig {
            region: original,
            ..JuliaConfig::default()
        };
        let zero_speed_motion = motion([1.0, 0.0], 0.0);

        let zero_speed_report = step_flight(
            &mut zero_speed_config,
            &zero_speed_motion,
            1.0,
            &FlightLimits::default(),
        );

        assert_eq!(zero_speed_config.region, original);
        assert!(!zero_speed_report.clamped);
        assert_eq!(zero_speed_report.warning, None);
    }

    #[test]
    fn same_speed_moves_more_at_wider_zoom() {
        let mut narrow = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let mut wide = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);

        step_flight(&mut narrow, &motion, 0.5, &FlightLimits::default());
        step_flight(&mut wide, &motion, 0.5, &FlightLimits::default());

        let narrow_delta = narrow.region.top_left().real - (-1.0);
        let wide_delta = wide.region.top_left().real - (-2.0);

        assert_approx_eq(narrow_delta, 1.0);
        assert_approx_eq(wide_delta, 2.0);
    }

    #[test]
    fn non_square_region_uses_independent_width_and_height() {
        let mut config = JuliaConfig {
            region: rect(-3.0, -1.0, 1.0, 2.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, -1.0], 0.5);

        step_flight(&mut config, &motion, 1.0, &FlightLimits::default());

        assert_approx_eq(config.region.top_left().real, -1.0);
        assert_approx_eq(config.region.bottom_right().real, 3.0);
        assert_approx_eq(config.region.top_left().imag, -2.5);
        assert_approx_eq(config.region.bottom_right().imag, 0.5);
    }

    #[test]
    fn center_clamp_limits_real_and_preserves_dimensions() {
        let limits = FlightLimits {
            max_center_abs: 1.0,
            ..FlightLimits::default()
        };
        let mut config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);

        let report = step_flight(&mut config, &motion, 1.0, &limits);

        assert_approx_eq(config.region.width(), 2.0);
        assert_approx_eq(config.region.height(), 2.0);
        assert_approx_eq(
            (config.region.top_left().real + config.region.bottom_right().real) * 0.5,
            1.0,
        );
        assert!(report.clamped);
        assert_eq!(report.warning, Some(FlightWarning::CenterClamped));
    }

    #[test]
    fn center_clamp_limits_imag_and_both_axes() {
        let limits = FlightLimits {
            max_center_abs: 1.0,
            ..FlightLimits::default()
        };

        let mut imag_config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let imag_motion = motion([0.0, 1.0], 1.0);
        step_flight(&mut imag_config, &imag_motion, 1.0, &limits);
        assert_approx_eq(
            (imag_config.region.top_left().imag + imag_config.region.bottom_right().imag) * 0.5,
            1.0,
        );

        let mut both_config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let both_motion = motion([1.0, 1.0], 1.0);
        step_flight(&mut both_config, &both_motion, 1.0, &limits);
        assert_approx_eq(
            (both_config.region.top_left().real + both_config.region.bottom_right().real) * 0.5,
            1.0,
        );
        assert_approx_eq(
            (both_config.region.top_left().imag + both_config.region.bottom_right().imag) * 0.5,
            1.0,
        );
    }

    #[test]
    fn extent_clamp_scales_up_and_down_with_aspect_ratio_preserved() {
        let up_limits = FlightLimits {
            min_region_extent: 3.0,
            max_region_extent: 10.0,
            ..FlightLimits::default()
        };
        let mut up_config = JuliaConfig {
            region: rect(-1.0, -2.0, 1.0, 2.0),
            ..JuliaConfig::default()
        };
        let up_motion = motion([1.0, 0.0], 1.0);

        let up_report = step_flight(&mut up_config, &up_motion, 0.0, &up_limits);

        assert!(up_report.clamped);
        assert_eq!(up_report.warning, Some(FlightWarning::ExtentClamped));
        assert_approx_eq(up_config.region.width(), 3.0);
        assert_approx_eq(up_config.region.height(), 6.0);

        let down_limits = FlightLimits {
            min_region_extent: 0.1,
            max_region_extent: 3.0,
            ..FlightLimits::default()
        };
        let mut down_config = JuliaConfig {
            region: rect(-3.0, -2.0, 3.0, 2.0),
            ..JuliaConfig::default()
        };
        let down_motion = motion([1.0, 0.0], 1.0);

        let down_report = step_flight(&mut down_config, &down_motion, 0.0, &down_limits);

        assert!(down_report.clamped);
        assert_eq!(down_report.warning, Some(FlightWarning::ExtentClamped));
        assert_approx_eq(down_config.region.width(), 3.0);
        assert_approx_eq(down_config.region.height(), 2.0);
    }

    #[test]
    fn non_finite_region_resets_to_default_for_nan_and_infinity() {
        let mut nan_config = JuliaConfig {
            region: rect(f64::NAN, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);

        let nan_report = step_flight(&mut nan_config, &motion, 1.0, &FlightLimits::default());

        assert_eq!(nan_config.region, default_region());
        assert!(nan_report.clamped);
        assert_eq!(nan_report.warning, Some(FlightWarning::NonFiniteReset));

        let mut inf_config = JuliaConfig {
            region: rect(-1.0, -1.0, f64::INFINITY, 1.0),
            ..JuliaConfig::default()
        };

        let inf_report = step_flight(&mut inf_config, &motion, 1.0, &FlightLimits::default());

        assert_eq!(inf_config.region, default_region());
        assert!(inf_report.clamped);
        assert_eq!(inf_report.warning, Some(FlightWarning::NonFiniteReset));
    }
}
