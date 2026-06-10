use crate::{
    controllers::interactive::data::fractal_config::FractalConfig,
    core::{
        data::{
            complex::Complex, complex_rect::ComplexRect, deep_region::DeepRegion,
            pixel_rect::PixelRect,
        },
        fractals::mandelbrot::{
            algorithm::MandelbrotAlgorithm,
            colour_mapping::{
                factory::mandelbrot_colour_map_factory, kinds::MandelbrotColourMapKinds,
            },
            perturbation::{algorithm::MandelbrotPerturbationAlgorithm, orbit_cache::OrbitCache},
            render_path::MandelbrotRenderPath,
        },
    },
};
use std::sync::Arc;

const DEFAULT_MAX_ITERATIONS: u32 = 800;

/// Below this view extent the direct f64 algorithm runs out of mantissa for
/// per-pixel coordinates and rendering switches to perturbation.
pub const PERTURBATION_EXTENT_THRESHOLD: f64 = 1e-8;

pub(crate) fn default_region() -> DeepRegion {
    let rect = ComplexRect::new(
        Complex {
            real: -2.5,
            imag: -1.0,
        },
        Complex {
            real: 1.0,
            imag: 1.0,
        },
    )
    .expect("default fractal region is valid");

    DeepRegion::from_complex_rect(&rect)
}

#[derive(Debug, Clone)]
pub struct MandelbrotConfig {
    pub region: DeepRegion,
    pub max_iterations: u32,
    pub colour_map_kind: MandelbrotColourMapKinds,
    pub orbit_cache: Arc<OrbitCache>,
}

impl Default for MandelbrotConfig {
    fn default() -> Self {
        Self {
            region: default_region(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            colour_map_kind: MandelbrotColourMapKinds::default(),
            orbit_cache: Arc::new(OrbitCache::new()),
        }
    }
}

impl PartialEq for MandelbrotConfig {
    fn eq(&self, other: &Self) -> bool {
        // The orbit cache is shared infrastructure, not view state.
        self.region == other.region
            && self.max_iterations == other.max_iterations
            && self.colour_map_kind == other.colour_map_kind
    }
}

impl MandelbrotConfig {
    pub(crate) fn build_render_request(&self, pixel_rect: PixelRect) -> FractalConfig {
        let colour_map = mandelbrot_colour_map_factory(self.colour_map_kind, self.max_iterations);

        let algorithm = if self.uses_perturbation() {
            MandelbrotRenderPath::Perturbation(
                MandelbrotPerturbationAlgorithm::new(
                    pixel_rect,
                    self.region.clone(),
                    self.max_iterations,
                    Arc::clone(&self.orbit_cache),
                )
                .expect("mandelbrot perturbation settings should be valid"),
            )
        } else {
            let region = self
                .region
                .to_complex_rect()
                .expect("region above the perturbation threshold cannot collapse in f64");

            MandelbrotRenderPath::Direct(
                MandelbrotAlgorithm::new(pixel_rect, region, self.max_iterations)
                    .expect("mandelbrot algorithm settings should be valid"),
            )
        };

        FractalConfig::Mandelbrot {
            colour_map,
            algorithm,
        }
    }

    #[must_use]
    pub fn uses_perturbation(&self) -> bool {
        self.region.min_extent() <= PERTURBATION_EXTENT_THRESHOLD
    }

    pub(crate) fn reset_view(&mut self) {
        self.region = default_region();
        self.max_iterations = DEFAULT_MAX_ITERATIONS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shallow_zoom_uses_the_direct_algorithm() {
        let config = MandelbrotConfig::default();
        let pixel_rect = PixelRect::new(
            crate::core::data::point::Point { x: 0, y: 0 },
            crate::core::data::point::Point { x: 7, y: 7 },
        )
        .unwrap();

        let request = config.build_render_request(pixel_rect);

        assert!(matches!(
            request,
            FractalConfig::Mandelbrot {
                algorithm: MandelbrotRenderPath::Direct(_),
                ..
            }
        ));
    }

    #[test]
    fn deep_zoom_switches_to_perturbation() {
        let mut config = MandelbrotConfig::default();
        config.region = config.region.with_extent(1e-12, 1e-12).unwrap();

        let pixel_rect = PixelRect::new(
            crate::core::data::point::Point { x: 0, y: 0 },
            crate::core::data::point::Point { x: 7, y: 7 },
        )
        .unwrap();

        let request = config.build_render_request(pixel_rect);

        assert!(matches!(
            request,
            FractalConfig::Mandelbrot {
                algorithm: MandelbrotRenderPath::Perturbation(_),
                ..
            }
        ));
    }

    #[test]
    fn equality_ignores_the_orbit_cache() {
        let a = MandelbrotConfig::default();
        let mut b = MandelbrotConfig::default();
        b.orbit_cache = Arc::new(OrbitCache::new());

        assert_eq!(a, b);

        b.max_iterations += 1;
        assert_ne!(a, b);
    }
}
