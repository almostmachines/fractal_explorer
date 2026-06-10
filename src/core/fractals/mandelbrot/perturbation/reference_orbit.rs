use crate::core::actions::cancellation::{CancelToken, Cancelled};
use crate::core::data::deep_complex::DeepComplex;

const CANCEL_CHECK_INTERVAL_ITERATIONS: u32 = 64;
const ESCAPE_RADIUS_SQ: f64 = 4.0;

/// A reference orbit `Z_0, Z_1, ...` for `Z' = Z² + C` computed at high
/// precision, snapshotted to f64 pairs for the per-pixel delta iteration.
///
/// The orbit ends either at the first escaped value (`|Z| > 2`) or after
/// `max_iterations` steps when the reference point is interior.
#[derive(Debug)]
pub struct ReferenceOrbit {
    point: DeepComplex,
    precision_bits: usize,
    max_iterations: u32,
    escaped: bool,
    orbit: Vec<[f64; 2]>,
}

impl ReferenceOrbit {
    pub fn compute<C: CancelToken + ?Sized>(
        point: &DeepComplex,
        max_iterations: u32,
        precision_bits: usize,
        cancel: &C,
    ) -> Result<Self, Cancelled> {
        let precision_bits = precision_bits.max(64);
        let point = point.with_precision(precision_bits);
        let c_re = &point.re;
        let c_im = &point.im;

        let mut z_re = dashu_float::FBig::ZERO.with_precision(precision_bits).value();
        let mut z_im = z_re.clone();

        let mut orbit = Vec::with_capacity((max_iterations as usize).saturating_add(1).min(1 << 20));
        orbit.push([0.0f64, 0.0f64]);

        let mut escaped = false;

        for n in 1..=max_iterations {
            if n % CANCEL_CHECK_INTERVAL_ITERATIONS == 0 && cancel.is_cancelled() {
                return Err(Cancelled);
            }

            let zr2 = z_re.sqr();
            let zi2 = z_im.sqr();
            let cross = &z_re * &z_im;

            z_re = zr2 - zi2 + c_re;
            z_im = &cross + &cross + c_im;

            let re = z_re.to_f64().value();
            let im = z_im.to_f64().value();
            orbit.push([re, im]);

            if re * re + im * im > ESCAPE_RADIUS_SQ {
                escaped = true;
                break;
            }
        }

        Ok(Self {
            point,
            precision_bits,
            max_iterations,
            escaped,
            orbit,
        })
    }

    #[must_use]
    pub fn point(&self) -> &DeepComplex {
        &self.point
    }

    #[must_use]
    pub fn orbit(&self) -> &[[f64; 2]] {
        &self.orbit
    }

    #[must_use]
    pub fn escaped(&self) -> bool {
        self.escaped
    }

    /// Whether this orbit can serve a render at the given point, iteration
    /// cap and precision.
    ///
    /// An escaped orbit is complete — it covers any iteration cap. An
    /// interior orbit only covers caps up to the one it was computed with.
    #[must_use]
    pub fn covers(
        &self,
        point: &DeepComplex,
        max_iterations: u32,
        precision_bits: usize,
    ) -> bool {
        self.precision_bits >= precision_bits
            && (self.escaped || self.max_iterations >= max_iterations)
            && self.point == *point
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::cancellation::NeverCancel;

    fn deep(re: f64, im: f64) -> DeepComplex {
        DeepComplex::from_f64(re, im).unwrap()
    }

    #[test]
    fn interior_point_runs_to_max_iterations() {
        let orbit = ReferenceOrbit::compute(&deep(0.0, 0.0), 100, 128, &NeverCancel).unwrap();

        assert!(!orbit.escaped());
        assert_eq!(orbit.orbit().len(), 101);
        assert!(orbit.orbit().iter().all(|z| z == &[0.0, 0.0]));
    }

    #[test]
    fn escaping_point_stops_at_escape() {
        // c = 3: Z_1 = 3, |Z_1|² = 9 > 4 — escape on the first step.
        let orbit = ReferenceOrbit::compute(&deep(3.0, 0.0), 100, 128, &NeverCancel).unwrap();

        assert!(orbit.escaped());
        assert_eq!(orbit.orbit().len(), 2);
        assert_eq!(orbit.orbit()[0], [0.0, 0.0]);
        assert_eq!(orbit.orbit()[1], [3.0, 0.0]);
    }

    #[test]
    fn orbit_matches_f64_iteration_at_moderate_precision() {
        let (c_re, c_im) = (-0.1, 0.65);
        let reference = ReferenceOrbit::compute(&deep(c_re, c_im), 50, 256, &NeverCancel).unwrap();

        let (mut zr, mut zi) = (0.0f64, 0.0f64);
        for (n, snapshot) in reference.orbit().iter().enumerate().skip(1) {
            let new_zr = zr * zr - zi * zi + c_re;
            let new_zi = 2.0 * zr * zi + c_im;
            zr = new_zr;
            zi = new_zi;

            assert!(
                (snapshot[0] - zr).abs() <= 1e-9 && (snapshot[1] - zi).abs() <= 1e-9,
                "orbit diverged from f64 reference at n={n}: {snapshot:?} vs ({zr}, {zi})"
            );
        }
    }

    #[test]
    fn compute_honours_cancellation() {
        let cancel = || true;
        let result = ReferenceOrbit::compute(&deep(0.0, 0.0), 1000, 128, &cancel);

        assert!(matches!(result, Err(Cancelled)));
    }

    #[test]
    fn covers_requires_same_point_and_sufficient_precision() {
        // c = 0 is interior, so the orbit runs to the full iteration cap.
        let point = deep(0.0, 0.0);
        let orbit = ReferenceOrbit::compute(&point, 100, 256, &NeverCancel).unwrap();

        assert!(!orbit.escaped());
        assert!(orbit.covers(&point, 100, 256));
        assert!(orbit.covers(&point, 50, 128));
        assert!(!orbit.covers(&point, 200, 256), "interior orbit cannot serve a higher cap");
        assert!(!orbit.covers(&point, 100, 512), "higher precision demands a recompute");
        assert!(!orbit.covers(&deep(-0.75, 0.2), 100, 256));
    }

    #[test]
    fn escaped_orbit_covers_any_iteration_cap() {
        let point = deep(3.0, 0.0);
        let orbit = ReferenceOrbit::compute(&point, 10, 128, &NeverCancel).unwrap();

        assert!(orbit.escaped());
        assert!(orbit.covers(&point, 1_000_000, 128));
    }
}
