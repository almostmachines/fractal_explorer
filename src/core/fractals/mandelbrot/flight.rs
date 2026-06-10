use crate::core::{
    data::pixel_rect::PixelRect,
    flight::{FlightLimits, FlightUpdateReport, FlightWarning, MotionState},
    fractals::mandelbrot::mandelbrot_config::{MandelbrotConfig, default_region},
};

pub fn step_flight(
    config: &mut MandelbrotConfig,
    motion: &MotionState,
    dt: f64,
    limits: &FlightLimits,
) -> FlightUpdateReport {
    step_flight_in_viewport(config, motion, dt, limits, None)
}

/// Advances the Mandelbrot view by one flight tick.
///
/// The view is a `DeepRegion` (arbitrary-precision centre, f64 extents), so
/// unlike the f64 viewport-precision floor used for Julia, zooming is only
/// limited by the f64 exponent range of the extent itself
/// (`FlightLimits::min_region_extent`, ~1e-280 by default). The viewport is
/// accepted for signature parity but not needed: perturbation rendering
/// keeps adjacent pixels distinct at any permitted depth.
pub fn step_flight_in_viewport(
    config: &mut MandelbrotConfig,
    motion: &MotionState,
    dt: f64,
    limits: &FlightLimits,
    _viewport: Option<PixelRect>,
) -> FlightUpdateReport {
    let mut report = FlightUpdateReport::default();

    if motion.paused || motion.speed_world_per_sec == 0.0 {
        return report;
    }

    let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

    if !scale.is_finite() || scale <= 0.0 || !limits.steer_strength.is_finite() || !dt.is_finite()
    {
        reset_non_finite(config, &mut report);
        return report;
    }

    let width = config.region.width();
    let height = config.region.height();

    // Pan distance follows the current view size so steering feels the same
    // at every depth.
    let pan_re = motion.heading[0] * limits.steer_strength * width * dt;
    let pan_im = motion.heading[1] * limits.steer_strength * height * dt;

    let mut new_width = width * scale;
    let mut new_height = height * scale;

    if !new_width.is_finite() || !new_height.is_finite() || new_width <= 0.0 || new_height <= 0.0
    {
        reset_non_finite(config, &mut report);
        return report;
    }

    let max_extent = limits.min_region_extent.max(limits.max_region_extent);
    let min_extent = limits
        .min_region_extent
        .min(limits.max_region_extent)
        .max(0.0);

    let extent_scale = if new_width < min_extent || new_height < min_extent {
        let width_scale = if new_width < min_extent {
            min_extent / new_width
        } else {
            1.0
        };
        let height_scale = if new_height < min_extent {
            min_extent / new_height
        } else {
            1.0
        };
        width_scale.max(height_scale)
    } else if new_width > max_extent || new_height > max_extent {
        let width_scale = if new_width > max_extent {
            max_extent / new_width
        } else {
            1.0
        };
        let height_scale = if new_height > max_extent {
            max_extent / new_height
        } else {
            1.0
        };
        width_scale.min(height_scale)
    } else {
        1.0
    };

    let extent_clamped = extent_scale != 1.0;
    if extent_clamped {
        new_width *= extent_scale;
        new_height *= extent_scale;
    }

    if !new_width.is_finite() || !new_height.is_finite() || new_width <= 0.0 || new_height <= 0.0
    {
        reset_non_finite(config, &mut report);
        return report;
    }

    // Resize, grow centre precision to suit the new depth, then pan — in
    // that order, so the pan offset is not swallowed by rounding.
    let resized = match config.region.with_extent(new_width, new_height) {
        Ok(region) => region.normalised(),
        Err(_) => {
            reset_non_finite(config, &mut report);
            return report;
        }
    };

    let Some(mut region) = resized.panned_by(pan_re, pan_im) else {
        reset_non_finite(config, &mut report);
        return report;
    };

    let max_center_abs = limits.max_center_abs.abs();
    let (centre_re, centre_im) = region.centre().to_f64();
    let clamped_re = centre_re.clamp(-max_center_abs, max_center_abs);
    let clamped_im = centre_im.clamp(-max_center_abs, max_center_abs);

    if clamped_re != centre_re {
        let Some(centre) = region.centre().with_re_f64(clamped_re) else {
            reset_non_finite(config, &mut report);
            return report;
        };
        region = region.with_centre(centre);
        mark_warning(&mut report, FlightWarning::CenterClamped);
    }

    if clamped_im != centre_im {
        let Some(centre) = region.centre().with_im_f64(clamped_im) else {
            reset_non_finite(config, &mut report);
            return report;
        };
        region = region.with_centre(centre);
        mark_warning(&mut report, FlightWarning::CenterClamped);
    }

    if extent_clamped {
        mark_warning(&mut report, FlightWarning::ExtentClamped);
    }

    config.region = region;
    report
}

fn mark_warning(report: &mut FlightUpdateReport, warning: FlightWarning) {
    report.clamped = true;
    report.warning = Some(warning);
}

fn reset_non_finite(config: &mut MandelbrotConfig, report: &mut FlightUpdateReport) {
    config.region = default_region();
    mark_warning(report, FlightWarning::NonFiniteReset);
}

#[cfg(test)]
mod tests {
    use super::{step_flight, step_flight_in_viewport};
    use crate::core::{
        data::{deep_complex::DeepComplex, deep_region::DeepRegion},
        flight::{FlightLimits, FlightWarning, MotionState},
        fractals::mandelbrot::mandelbrot_config::{MandelbrotConfig, default_region},
    };

    const EPSILON: f64 = 1e-12;

    fn region(centre_re: f64, centre_im: f64, width: f64, height: f64) -> DeepRegion {
        DeepRegion::new(
            DeepComplex::from_f64(centre_re, centre_im).expect("test centre is finite"),
            width,
            height,
        )
        .expect("test region should be valid")
        .normalised()
    }

    fn config_with(region: DeepRegion) -> MandelbrotConfig {
        MandelbrotConfig {
            region,
            ..MandelbrotConfig::default()
        }
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

    fn assert_centre(config: &MandelbrotConfig, expected_re: f64, expected_im: f64) {
        let (re, im) = config.region.centre().to_f64();
        assert_approx_eq(re, expected_re);
        assert_approx_eq(im, expected_im);
    }

    #[test]
    fn positive_speed_zooms_in() {
        let limits = FlightLimits {
            steer_strength: 0.0,
            ..FlightLimits::default()
        };
        let mut config = config_with(region(0.0, 0.0, 4.0, 2.0));
        let motion = motion([1.0, 0.0], 1.0);
        let dt = 0.5;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

        let report = step_flight(&mut config, &motion, dt, &limits);

        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
        assert_centre(&config, 0.0, 0.0);
        assert!(!report.clamped);
        assert_eq!(report.warning, None);
    }

    #[test]
    fn negative_speed_zooms_out() {
        let limits = FlightLimits {
            steer_strength: 0.0,
            ..FlightLimits::default()
        };
        let mut config = config_with(region(0.0, 0.0, 4.0, 2.0));
        let motion = motion([1.0, 0.0], -1.0);
        let dt = 0.5;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);

        step_flight(&mut config, &motion, dt, &limits);

        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
        assert_centre(&config, 0.0, 0.0);
    }

    #[test]
    fn steering_changes_centre_while_zooming() {
        let limits = FlightLimits::default();
        let mut config = config_with(region(0.0, 0.0, 4.0, 2.0));
        let motion = motion([1.0, 0.0], 1.0);
        let dt = 1.0;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);
        let expected_centre_shift = dt * limits.steer_strength * 4.0;

        step_flight(&mut config, &motion, dt, &limits);

        assert_centre(&config, expected_centre_shift, 0.0);
        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
    }

    #[test]
    fn negative_speed_pans_same_direction_as_heading() {
        let limits = FlightLimits::default();
        let mut config = config_with(region(0.0, 0.0, 4.0, 2.0));
        let motion = motion([1.0, 0.0], -1.0);
        let dt = 1.0;
        let scale = limits.zoom_base.powf(-motion.speed_world_per_sec * dt);
        let expected_centre_shift = dt * limits.steer_strength * 4.0;

        step_flight(&mut config, &motion, dt, &limits);

        assert!(expected_centre_shift > 0.0);
        assert_centre(&config, expected_centre_shift, 0.0);
        assert_approx_eq(config.region.width(), 4.0 * scale);
        assert_approx_eq(config.region.height(), 2.0 * scale);
    }

    #[test]
    fn paused_or_zero_speed_is_a_noop() {
        let original = region(0.0, 0.0, 4.0, 2.0);
        let mut paused_config = config_with(original.clone());
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

        let mut zero_speed_config = config_with(original.clone());
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
    fn centre_clamp_limits_real_and_preserves_dimensions() {
        let limits = FlightLimits {
            max_center_abs: 0.2,
            steer_strength: 0.5,
            ..FlightLimits::default()
        };
        let mut config = config_with(region(0.0, 0.0, 2.0, 2.0));
        let motion = motion([1.0, 0.0], 1.0);

        let report = step_flight(&mut config, &motion, 1.0, &limits);

        assert_approx_eq(config.region.width(), 1.0);
        assert_approx_eq(config.region.height(), 1.0);
        assert_centre(&config, 0.2, 0.0);
        assert!(report.clamped);
        assert_eq!(report.warning, Some(FlightWarning::CenterClamped));
    }

    #[test]
    fn centre_clamp_limits_imag_and_both_axes() {
        let limits = FlightLimits {
            max_center_abs: 0.2,
            steer_strength: 0.5,
            ..FlightLimits::default()
        };

        let mut imag_config = config_with(region(0.0, 0.0, 2.0, 2.0));
        let imag_motion = motion([0.0, 1.0], 1.0);
        step_flight(&mut imag_config, &imag_motion, 1.0, &limits);
        assert_centre(&imag_config, 0.0, 0.2);

        let mut both_config = config_with(region(0.0, 0.0, 2.0, 2.0));
        let both_motion = motion([1.0, 1.0], 1.0);
        step_flight(&mut both_config, &both_motion, 1.0, &limits);
        assert_centre(&both_config, 0.2, 0.2);
    }

    #[test]
    fn extent_clamp_scales_up_and_down_with_aspect_ratio_preserved() {
        let up_limits = FlightLimits {
            min_region_extent: 3.0,
            max_region_extent: 10.0,
            ..FlightLimits::default()
        };
        let mut up_config = config_with(region(0.0, 0.0, 2.0, 4.0));
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
        let mut down_config = config_with(region(0.0, 0.0, 6.0, 4.0));
        let down_motion = motion([1.0, 0.0], 1.0);

        let down_report = step_flight(&mut down_config, &down_motion, 0.0, &down_limits);

        assert!(down_report.clamped);
        assert_eq!(down_report.warning, Some(FlightWarning::ExtentClamped));
        assert_approx_eq(down_config.region.width(), 3.0);
        assert_approx_eq(down_config.region.height(), 2.0);
    }

    #[test]
    fn non_finite_speed_resets_to_default() {
        let mut config = config_with(region(0.0, 0.0, 4.0, 2.0));
        let inf_motion = motion([1.0, 0.0], f64::NEG_INFINITY);

        let report = step_flight(&mut config, &inf_motion, 1.0, &FlightLimits::default());

        assert_eq!(config.region, default_region());
        assert!(report.clamped);
        assert_eq!(report.warning, Some(FlightWarning::NonFiniteReset));
    }

    #[test]
    fn deep_zoom_is_not_clamped_until_the_extent_floor() {
        // The old f64 pipeline had to clamp around 1e-11 to keep pixel rows
        // distinct; perturbation rendering needs no such guard. A view at
        // 1e-100 must keep zooming freely.
        let limits = FlightLimits::default();
        let viewport = crate::core::data::pixel_rect::PixelRect::new(
            crate::core::data::point::Point { x: 0, y: 0 },
            crate::core::data::point::Point { x: 1919, y: 1079 },
        )
        .expect("viewport should be valid");
        let mut config = config_with(region(-0.75, 0.1, 1e-100, 1e-100));
        let motion = motion([0.0, 0.0], 1.0);

        let report = step_flight_in_viewport(&mut config, &motion, 0.5, &limits, Some(viewport));

        assert!(!report.clamped, "deep zoom must not clamp above the floor");
        assert!(config.region.width() < 1e-100);
    }

    #[test]
    fn extent_floor_clamps_at_the_deep_limit() {
        let limits = FlightLimits::default();
        let floor = limits.min_region_extent;
        let mut config = config_with(region(-0.75, 0.1, floor * 1.5, floor * 1.5));
        let motion = motion([0.0, 0.0], 5.0);

        // Zoom hard enough to cross the floor in one step.
        let report = step_flight(&mut config, &motion, 1.0, &limits);

        assert!(report.clamped);
        assert_eq!(report.warning, Some(FlightWarning::ExtentClamped));
        assert_approx_eq(config.region.width() / floor, 1.0);
    }

    #[test]
    fn panning_at_depth_moves_centre_below_f64_resolution() {
        let limits = FlightLimits {
            steer_strength: 0.5,
            ..FlightLimits::default()
        };
        let start = region(-0.75, 0.1, 1e-40, 1e-40);
        let mut config = config_with(start.clone());
        let motion = motion([1.0, 0.0], 0.0001);

        let report = step_flight(&mut config, &motion, 1.0, &limits);

        assert!(!report.clamped);
        assert_ne!(config.region.centre(), start.centre());

        let (dre, dim) = config.region.centre().sub_to_f64(start.centre());
        let expected = 0.5 * 1e-40; // heading * steer * width * dt
        assert!(
            (dre - expected).abs() < expected * 1e-9,
            "pan was lost to rounding: dre={dre}"
        );
        assert_eq!(dim, 0.0);
    }

    #[test]
    fn precision_grows_as_flight_dives_deeper() {
        let limits = FlightLimits::default();
        let mut config = config_with(region(-0.75, 0.1, 1e-3, 1e-3));
        let before_bits = config.region.centre().precision_bits();
        let motion = motion([0.1, 0.1], 5.0);

        for _ in 0..200 {
            let report = step_flight(&mut config, &motion, 0.25, &limits);
            assert_ne!(report.warning, Some(FlightWarning::NonFiniteReset));
        }

        assert!(config.region.width() < 1e-40);
        assert!(
            config.region.centre().precision_bits() > before_bits,
            "centre precision must grow with depth"
        );
        assert!(
            config.region.centre().precision_bits() >= config.region.required_precision_bits()
        );
    }
}
