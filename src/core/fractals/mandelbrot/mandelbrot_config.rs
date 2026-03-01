use crate::{
    controllers::interactive::data::fractal_config::FractalConfig,
    core::{
        data::{complex::Complex, complex_rect::ComplexRect, pixel_rect::PixelRect},
        fractals::mandelbrot::{
            algorithm::MandelbrotAlgorithm,
            colour_mapping::{
                factory::mandelbrot_colour_map_factory, kinds::MandelbrotColourMapKinds,
            },
        },
    },
};

const DEFAULT_MAX_ITERATIONS: u32 = 256;

pub(crate) fn default_region() -> ComplexRect {
    ComplexRect::new(
        Complex {
            real: -2.5,
            imag: -1.0,
        },
        Complex {
            real: 1.0,
            imag: 1.0,
        },
    )
    .expect("default fractal region is valid")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MandelbrotConfig {
    pub region: ComplexRect,
    pub max_iterations: u32,
    pub colour_map_kind: MandelbrotColourMapKinds,
}

impl Default for MandelbrotConfig {
    fn default() -> Self {
        Self {
            region: default_region(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            colour_map_kind: MandelbrotColourMapKinds::default(),
        }
    }
}

impl MandelbrotConfig {
    pub(crate) fn build_render_request(&self, pixel_rect: PixelRect) -> FractalConfig {
        let colour_map = mandelbrot_colour_map_factory(self.colour_map_kind, self.max_iterations);
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, self.region, self.max_iterations)
            .expect("mandelbrot algorithm settings should be valid");

        FractalConfig::Mandelbrot {
            colour_map,
            algorithm,
        }
    }

    pub(crate) fn reset_view(&mut self) {
        self.region = default_region();
        self.max_iterations = DEFAULT_MAX_ITERATIONS;
    }
}
