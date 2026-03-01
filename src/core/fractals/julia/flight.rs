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

    let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

    if let Some(region) =
        scaled_region_about_focal(&config.region, scale, motion.heading, limits.steer_strength)
    {
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

fn scaled_region_about_focal(
    region: &ComplexRect,
    scale: f64,
    heading: [f64; 2],
    steer_strength: f64,
) -> Option<ComplexRect> {
    if !scale.is_finite() || scale <= 0.0 || !steer_strength.is_finite() {
        return None;
    }

    let width = region.width();
    let height = region.height();
    let (center_real, center_imag) = region_center(region);

    let offset_real = heading[0] * steer_strength * width;
    let offset_imag = heading[1] * steer_strength * height;
    let center_scale = 1.0 - scale;

    let new_center_real = center_real + (center_scale * offset_real);
    let new_center_imag = center_imag + (center_scale * offset_imag);
    let new_width = width * scale;
    let new_height = height * scale;

    rebuild_region(new_center_real, new_center_imag, new_width, new_height)
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
    use super::{region_center, step_flight};
    use crate::core::{
        data::{complex::Complex, complex_rect::ComplexRect},
        flight::{FlightLimits, FlightWarning, MotionState},
        fractals::julia::julia_config::{JuliaConfig, default_region},
    };

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

    fn assert_region_center(region: &ComplexRect, expected_real: f64, expected_imag: f64) {
        let (center_real, center_imag) = region_center(region);
        assert_approx_eq(center_real, expected_real);
        assert_approx_eq(center_imag, expected_imag);
    }

    #[test]
    fn positive_speed_zooms_in() {
        let limits = FlightLimits {
            steer_strength: 0.0,
            ..FlightLimits::default()
        };
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);
        let dt = 0.5;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

        let report = step_flight(&mut config, &motion, dt, &limits);

        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
        assert_region_center(&config.region, 0.0, 0.0);
        assert!(!report.clamped);
        assert_eq!(report.warning, None);
    }

    #[test]
    fn negative_speed_zooms_out() {
        let limits = FlightLimits {
            steer_strength: 0.0,
            ..FlightLimits::default()
        };
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], -1.0);
        let dt = 0.5;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

        step_flight(&mut config, &motion, dt, &limits);

        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
        assert_region_center(&config.region, 0.0, 0.0);
    }

    #[test]
    fn steering_changes_center_while_zooming() {
        let limits = FlightLimits::default();
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);
        let dt = 1.0;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);
        let expected_center_shift = (1.0 - scale) * limits.steer_strength * 4.0;

        step_flight(&mut config, &motion, dt, &limits);

        assert_region_center(&config.region, expected_center_shift, 0.0);
        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
    }

    #[test]
    fn negative_speed_drifts_opposite_heading() {
        let limits = FlightLimits::default();
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], -1.0);
        let dt = 1.0;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);
        let expected_center_shift = (1.0 - scale) * limits.steer_strength * 4.0;

        step_flight(&mut config, &motion, dt, &limits);

        assert!(expected_center_shift < 0.0);
        assert_region_center(&config.region, expected_center_shift, 0.0);
        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
    }

    #[test]
    fn steer_strength_zero_means_zoom_about_center() {
        let limits = FlightLimits {
            steer_strength: 0.0,
            ..FlightLimits::default()
        };
        let mut config = JuliaConfig {
            region: rect(-2.0, -1.0, 2.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);
        let dt = 1.0;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

        step_flight(&mut config, &motion, dt, &limits);

        assert_region_center(&config.region, 0.0, 0.0);
        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
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
    fn center_clamp_limits_real_and_preserves_dimensions() {
        let limits = FlightLimits {
            max_center_abs: 0.2,
            ..FlightLimits::default()
        };
        let mut config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let motion = motion([1.0, 0.0], 1.0);

        let report = step_flight(&mut config, &motion, 1.0, &limits);

        assert_approx_eq(config.region.width(), 1.0);
        assert_approx_eq(config.region.height(), 1.0);
        assert_region_center(&config.region, 0.2, 0.0);
        assert!(report.clamped);
        assert_eq!(report.warning, Some(FlightWarning::CenterClamped));
    }

    #[test]
    fn center_clamp_limits_imag_and_both_axes() {
        let limits = FlightLimits {
            max_center_abs: 0.2,
            ..FlightLimits::default()
        };

        let mut imag_config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let imag_motion = motion([0.0, 1.0], 1.0);
        step_flight(&mut imag_config, &imag_motion, 1.0, &limits);
        assert_region_center(&imag_config.region, 0.0, 0.2);

        let mut both_config = JuliaConfig {
            region: rect(-1.0, -1.0, 1.0, 1.0),
            ..JuliaConfig::default()
        };
        let both_motion = motion([1.0, 1.0], 1.0);
        step_flight(&mut both_config, &both_motion, 1.0, &limits);
        assert_region_center(&both_config.region, 0.2, 0.2);
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
