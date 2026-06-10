use crate::core::actions::cancellation::{CancelToken, Cancelled, NeverCancel};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::deep_region::DeepRegion;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::mandelbrot::errors::mandelbrot::MandelbrotError;
use crate::core::fractals::mandelbrot::perturbation::orbit_cache::OrbitCache;
use crate::core::fractals::mandelbrot::perturbation::reference_orbit::ReferenceOrbit;
use crate::core::util::pixel_to_complex_coords::PixelToComplexCoordsError;
use std::sync::{Arc, OnceLock};

const ESCAPE_RADIUS_SQ: f64 = 4.0;

/// Mandelbrot rendering via perturbation theory.
///
/// One reference orbit is iterated at arbitrary precision (cached across
/// frames); every pixel then iterates only its small delta from that orbit
/// in plain f64:
///
/// ```text
/// δ' = 2·Z_m·δ + δ² + δc
/// ```
///
/// Rebasing (Zhuoran, fractalforums 2021) keeps a single reference glitch
/// free: whenever the full value `z = Z_m + δ` becomes smaller than `δ`
/// itself — or the reference orbit ends — the delta is rebased onto the
/// orbit start (`δ = z`, `m = 0`).
///
/// This pushes the usable zoom depth from f64's ~1e-13 down to the f64
/// exponent floor of the *extent* (~1e-290), because pixel deltas are tiny
/// relative offsets rather than absolute coordinates.
#[derive(Debug)]
pub struct MandelbrotPerturbationAlgorithm {
    pixel_rect: PixelRect,
    region: DeepRegion,
    max_iterations: u32,
    cache: Arc<OrbitCache>,
    orbit: OnceLock<Arc<ReferenceOrbit>>,
}

impl MandelbrotPerturbationAlgorithm {
    pub fn new(
        pixel_rect: PixelRect,
        region: DeepRegion,
        max_iterations: u32,
        cache: Arc<OrbitCache>,
    ) -> Result<Self, MandelbrotError> {
        if max_iterations == 0 {
            return Err(MandelbrotError::ZeroMaxIterationsError);
        }

        Ok(Self {
            pixel_rect,
            region: region.normalised(),
            max_iterations,
            cache,
            orbit: OnceLock::new(),
        })
    }

    #[must_use]
    pub fn region(&self) -> &DeepRegion {
        &self.region
    }

    #[must_use]
    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    /// Resolves the reference orbit (from cache or by computing it),
    /// honouring cancellation. Call this from the worker thread before
    /// rendering; the per-pixel methods then never block on orbit work.
    pub fn prepare<C: CancelToken + ?Sized>(&self, cancel: &C) -> Result<(), Cancelled> {
        if self.orbit.get().is_some() {
            return Ok(());
        }

        let orbit = self.cache.get_or_compute(
            self.region.centre(),
            self.max_iterations,
            self.region.required_precision_bits(),
            cancel,
        )?;

        let _ = self.orbit.set(orbit);
        Ok(())
    }

    /// Reference orbit snapshot, if `prepare` has run.
    #[must_use]
    pub fn orbit(&self) -> Option<&ReferenceOrbit> {
        self.orbit.get().map(Arc::as_ref)
    }

    fn orbit_or_compute(&self) -> &Arc<ReferenceOrbit> {
        self.orbit.get_or_init(|| {
            self.cache
                .get_or_compute(
                    self.region.centre(),
                    self.max_iterations,
                    self.region.required_precision_bits(),
                    &NeverCancel,
                )
                .expect("orbit computation cannot be cancelled with NeverCancel")
        })
    }

    /// Per-pixel offsets from the view centre, matching the linear
    /// top-left-to-bottom-right mapping of the direct algorithm.
    fn pixel_steps(&self) -> PixelSteps {
        let width_px = self.pixel_rect.width();
        let height_px = self.pixel_rect.height();

        let (step_re, half_width) = if width_px > 1 {
            let step = self.region.width() / f64::from(width_px - 1);
            (step, self.region.width() * 0.5)
        } else {
            (0.0, 0.0)
        };

        let (step_im, half_height) = if height_px > 1 {
            let step = self.region.height() / f64::from(height_px - 1);
            (step, self.region.height() * 0.5)
        } else {
            (0.0, 0.0)
        };

        // The reference point is allowed to differ from the view centre
        // (e.g. a reused orbit while panning); fold that offset into δc.
        let (origin_re, origin_im) = self
            .region
            .centre()
            .sub_to_f64(self.orbit_or_compute().point());

        PixelSteps {
            origin_re: origin_re - half_width,
            origin_im: origin_im - half_height,
            step_re,
            step_im,
        }
    }

    fn contains(&self, pixel: Point) -> bool {
        let top_left = self.pixel_rect.top_left();
        let bottom_right = self.pixel_rect.bottom_right();

        pixel.x >= top_left.x
            && pixel.x <= bottom_right.x
            && pixel.y >= top_left.y
            && pixel.y <= bottom_right.y
    }

    /// Iterates a single pixel's delta against the reference orbit,
    /// returning the escape iteration count (1..=max_iterations).
    fn iterate_delta(orbit: &[[f64; 2]], max_iterations: u32, dc_re: f64, dc_im: f64) -> u32 {
        debug_assert!(orbit.len() >= 2, "reference orbit needs Z_0 and Z_1");
        let last = orbit.len() - 1;

        let mut d_re = 0.0f64;
        let mut d_im = 0.0f64;
        let mut m = 0usize;

        for n in 1..=max_iterations {
            if m == last {
                // The reference orbit ended (escaped reference): rebase the
                // full value onto the orbit start.
                d_re += orbit[m][0];
                d_im += orbit[m][1];
                m = 0;
            }

            // δ' = (2·Z_m + δ)·δ + δc
            let sum_re = 2.0 * orbit[m][0] + d_re;
            let sum_im = 2.0 * orbit[m][1] + d_im;
            let new_re = sum_re * d_re - sum_im * d_im + dc_re;
            let new_im = sum_re * d_im + sum_im * d_re + dc_im;
            d_re = new_re;
            d_im = new_im;
            m += 1;

            let z_re = orbit[m][0] + d_re;
            let z_im = orbit[m][1] + d_im;
            let z_mag_sq = z_re * z_re + z_im * z_im;

            if z_mag_sq > ESCAPE_RADIUS_SQ {
                return n;
            }

            // Rebase when the full value drops below the delta: from here
            // the orbit start approximates the pixel better than the
            // reference tail does (this is what prevents glitches).
            if z_mag_sq < d_re * d_re + d_im * d_im {
                d_re = z_re;
                d_im = z_im;
                m = 0;
            }
        }

        max_iterations
    }
}

struct PixelSteps {
    origin_re: f64,
    origin_im: f64,
    step_re: f64,
    step_im: f64,
}

impl PixelSteps {
    #[inline]
    fn delta_c(&self, x_rel: i32, y_rel: i32) -> (f64, f64) {
        (
            self.origin_re + f64::from(x_rel) * self.step_re,
            self.origin_im + f64::from(y_rel) * self.step_im,
        )
    }
}

impl FractalAlgorithm for MandelbrotPerturbationAlgorithm {
    type Success = u32;
    type Failure = PixelToComplexCoordsError;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
        if !self.contains(pixel) {
            return Err(PixelToComplexCoordsError::PointOutsideRect {
                point: pixel,
                pixel_rect: self.pixel_rect,
            });
        }

        let orbit = Arc::clone(self.orbit_or_compute());
        let steps = self.pixel_steps();
        let top_left = self.pixel_rect.top_left();
        let (dc_re, dc_im) = steps.delta_c(pixel.x - top_left.x, pixel.y - top_left.y);

        Ok(Self::iterate_delta(
            orbit.orbit(),
            self.max_iterations,
            dc_re,
            dc_im,
        ))
    }

    fn compute_row_segment_into(
        &self,
        y: i32,
        x_start: i32,
        x_end: i32,
        output: &mut Vec<Self::Success>,
    ) -> Result<(), Self::Failure> {
        if x_start > x_end {
            return Ok(());
        }

        let top_left = self.pixel_rect.top_left();
        let bottom_right = self.pixel_rect.bottom_right();
        let in_bounds = y >= top_left.y
            && y <= bottom_right.y
            && x_start >= top_left.x
            && x_end <= bottom_right.x;

        if !in_bounds {
            for x in x_start..=x_end {
                output.push(self.compute(Point { x, y })?);
            }
            return Ok(());
        }

        let orbit = Arc::clone(self.orbit_or_compute());
        let orbit_values = orbit.orbit();
        let steps = self.pixel_steps();
        let y_rel = y - top_left.y;

        output.reserve((x_end - x_start + 1) as usize);

        for x in x_start..=x_end {
            let (dc_re, dc_im) = steps.delta_c(x - top_left.x, y_rel);
            output.push(Self::iterate_delta(
                orbit_values,
                self.max_iterations,
                dc_re,
                dc_im,
            ));
        }

        Ok(())
    }

    fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }
}

impl PartialEq for MandelbrotPerturbationAlgorithm {
    fn eq(&self, other: &Self) -> bool {
        self.pixel_rect == other.pixel_rect
            && self.max_iterations == other.max_iterations
            && self.region == other.region
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::deep_complex::DeepComplex;
    use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;

    fn pixel_rect(width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )
        .unwrap()
    }

    fn deep_region(re: f64, im: f64, extent: f64) -> DeepRegion {
        DeepRegion::new(DeepComplex::from_f64(re, im).unwrap(), extent, extent)
            .unwrap()
            .normalised()
    }

    fn perturbation(
        rect: PixelRect,
        region: DeepRegion,
        max_iterations: u32,
    ) -> MandelbrotPerturbationAlgorithm {
        MandelbrotPerturbationAlgorithm::new(
            rect,
            region,
            max_iterations,
            Arc::new(OrbitCache::new()),
        )
        .unwrap()
    }

    #[test]
    fn rejects_zero_max_iterations() {
        let result = MandelbrotPerturbationAlgorithm::new(
            pixel_rect(4, 4),
            deep_region(0.0, 0.0, 1.0),
            0,
            Arc::new(OrbitCache::new()),
        );

        assert!(matches!(result, Err(MandelbrotError::ZeroMaxIterationsError)));
    }

    #[test]
    fn out_of_bounds_pixel_errors_like_the_direct_algorithm() {
        let rect = pixel_rect(4, 4);
        let algorithm = perturbation(rect, deep_region(-0.75, 0.0, 1.0), 10);
        let point = Point { x: 4, y: 0 };

        assert_eq!(
            algorithm.compute(point),
            Err(PixelToComplexCoordsError::PointOutsideRect {
                point,
                pixel_rect: rect
            })
        );
    }

    #[test]
    fn matches_direct_f64_algorithm_at_moderate_zoom() {
        let extent = 1e-6;
        let (centre_re, centre_im) = (-0.74364388703715, 0.13182590420532);
        let max_iterations = 600;
        let rect = pixel_rect(96, 64);

        let region = deep_region(centre_re, centre_im, extent);
        let perturbed = perturbation(rect, region.clone(), max_iterations);

        let direct_rect = region.to_complex_rect().unwrap();
        let direct = MandelbrotAlgorithm::new(rect, direct_rect, max_iterations).unwrap();

        let mut total = 0usize;
        let mut mismatches = 0usize;
        let mut escaped = 0usize;
        let mut interior = 0usize;

        for y in 0..64 {
            let mut perturbed_row = Vec::new();
            let mut direct_row = Vec::new();
            perturbed
                .compute_row_segment_into(y, 0, 95, &mut perturbed_row)
                .unwrap();
            direct
                .compute_row_segment_into(y, 0, 95, &mut direct_row)
                .unwrap();

            for (p, d) in perturbed_row.iter().zip(direct_row.iter()) {
                total += 1;
                if p != d {
                    mismatches += 1;
                }
                if *p == max_iterations {
                    interior += 1;
                } else {
                    escaped += 1;
                }
            }
        }

        // The view straddles the set boundary, so both classes must appear
        // for this comparison to mean anything.
        assert!(escaped > 0, "expected escaped pixels in the test view");
        assert!(interior > 0, "expected interior pixels in the test view");

        let mismatch_fraction = mismatches as f64 / total as f64;
        assert!(
            mismatch_fraction < 0.01,
            "perturbation diverged from direct f64: {mismatches}/{total} pixels differ"
        );
    }

    #[test]
    fn forced_rebase_wraps_orbit_end_correctly() {
        // Reference c = 2 escapes after two steps (orbit [0, 2, 6]), while
        // the pixel at δc = -4 lands on c = -2, which never escapes. The
        // delta iteration must repeatedly rebase past the orbit end and
        // still classify the pixel as interior.
        let orbit = ReferenceOrbit::compute(
            &DeepComplex::from_f64(2.0, 0.0).unwrap(),
            100,
            128,
            &NeverCancel,
        )
        .unwrap();

        assert!(orbit.escaped());

        let result =
            MandelbrotPerturbationAlgorithm::iterate_delta(orbit.orbit(), 50, -4.0, 0.0);

        assert_eq!(result, 50, "c = -2 is in the set and must not escape");
    }

    #[test]
    fn renders_structure_far_beyond_f64_resolution() {
        // A single pixel row along the real axis across the needle tip at
        // c = -2: pixels left of the tip escape in ~log4(1/extent)
        // iterations (a logarithmic gradient), pixels on the needle
        // (re >= -2) never escape. At extent 1e-40 a plain f64 algorithm
        // would see every pixel as the same complex point.
        let rect = pixel_rect(64, 1);
        let algorithm = perturbation(rect, deep_region(-2.0, 0.0, 1e-40), 500);

        let mut row = Vec::new();
        algorithm.compute_row_segment_into(0, 0, 63, &mut row).unwrap();

        let interior = row.iter().filter(|&&v| v == 500).count();
        let escaped = row.len() - interior;
        let distinct: std::collections::BTreeSet<u32> = row.iter().copied().collect();

        assert!(interior > 0, "the needle must appear as interior pixels");
        assert!(escaped > 0, "pixels left of the tip must escape");
        assert!(
            distinct.len() >= 3,
            "expected an escape-time gradient, got {} distinct counts",
            distinct.len()
        );
    }

    #[test]
    fn renders_structure_at_extent_1e_200() {
        let rect = pixel_rect(48, 32);
        let algorithm = perturbation(rect, deep_region(-2.0, 0.0, 1e-200), 1500);

        let mut counts = std::collections::BTreeSet::new();

        for y in 0..32 {
            let mut row = Vec::new();
            algorithm.compute_row_segment_into(y, 0, 47, &mut row).unwrap();
            counts.extend(row);
        }

        assert!(
            counts.len() >= 10,
            "expected varied escape structure at 1e-200, got {} distinct counts",
            counts.len()
        );
    }

    #[test]
    fn prepare_honours_cancellation() {
        let algorithm = perturbation(pixel_rect(8, 8), deep_region(0.0, 0.0, 1e-20), 10_000);
        let cancel = || true;

        assert!(algorithm.prepare(&cancel).is_err());
        assert!(algorithm.orbit().is_none());

        assert!(algorithm.prepare(&NeverCancel).is_ok());
        assert!(algorithm.orbit().is_some());
    }

    #[test]
    fn compute_works_without_explicit_prepare() {
        let algorithm = perturbation(pixel_rect(8, 8), deep_region(-0.75, 0.0, 0.5), 100);

        let result = algorithm.compute(Point { x: 4, y: 4 });

        assert!(result.is_ok());
    }

    #[test]
    fn equality_ignores_orbit_state() {
        let region = deep_region(-0.75, 0.0, 1e-12);
        let a = perturbation(pixel_rect(8, 8), region.clone(), 100);
        let b = perturbation(pixel_rect(8, 8), region, 100);

        a.prepare(&NeverCancel).unwrap();

        assert_eq!(a, b);
    }
}
