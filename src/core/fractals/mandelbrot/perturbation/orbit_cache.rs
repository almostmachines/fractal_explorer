use crate::core::actions::cancellation::{CancelToken, Cancelled};
use crate::core::data::deep_complex::DeepComplex;
use crate::core::fractals::mandelbrot::perturbation::reference_orbit::ReferenceOrbit;
use std::sync::{Arc, Mutex};

/// Caches the most recent reference orbit so that consecutive frames reuse
/// it instead of recomputing.
///
/// During a pure zoom the centre does not move, so every frame is a cache
/// hit until the iteration cap or required precision outgrows the cached
/// orbit.
#[derive(Debug, Default)]
pub struct OrbitCache {
    cached: Mutex<Option<Arc<ReferenceOrbit>>>,
}

impl OrbitCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_compute<C: CancelToken + ?Sized>(
        &self,
        point: &DeepComplex,
        max_iterations: u32,
        precision_bits: usize,
        cancel: &C,
    ) -> Result<Arc<ReferenceOrbit>, Cancelled> {
        {
            let guard = self.cached.lock().unwrap();
            if let Some(orbit) = guard.as_ref() {
                if orbit.covers(point, max_iterations, precision_bits) {
                    return Ok(Arc::clone(orbit));
                }
            }
        }

        let orbit = Arc::new(ReferenceOrbit::compute(
            point,
            max_iterations,
            precision_bits,
            cancel,
        )?);

        let mut guard = self.cached.lock().unwrap();
        *guard = Some(Arc::clone(&orbit));

        Ok(orbit)
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
    fn repeated_requests_reuse_the_cached_orbit() {
        let cache = OrbitCache::new();
        let point = deep(-0.75, 0.1);

        let first = cache
            .get_or_compute(&point, 100, 128, &NeverCancel)
            .unwrap();
        let second = cache
            .get_or_compute(&point, 100, 128, &NeverCancel)
            .unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn lower_iteration_cap_still_hits_the_cache() {
        let cache = OrbitCache::new();
        let point = deep(-0.75, 0.1);

        let first = cache
            .get_or_compute(&point, 100, 128, &NeverCancel)
            .unwrap();
        let second = cache.get_or_compute(&point, 50, 128, &NeverCancel).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn different_point_recomputes() {
        let cache = OrbitCache::new();

        let first = cache
            .get_or_compute(&deep(-0.75, 0.1), 100, 128, &NeverCancel)
            .unwrap();
        let second = cache
            .get_or_compute(&deep(-0.74, 0.1), 100, 128, &NeverCancel)
            .unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn growing_precision_recomputes() {
        let cache = OrbitCache::new();
        let point = deep(-0.75, 0.1);

        let first = cache
            .get_or_compute(&point, 100, 128, &NeverCancel)
            .unwrap();
        let second = cache
            .get_or_compute(&point, 100, 256, &NeverCancel)
            .unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn cancellation_leaves_cache_usable() {
        let cache = OrbitCache::new();
        let point = deep(0.0, 0.0);
        let cancel = || true;

        assert!(cache.get_or_compute(&point, 1000, 128, &cancel).is_err());
        assert!(
            cache
                .get_or_compute(&point, 1000, 128, &NeverCancel)
                .is_ok()
        );
    }
}
