use crate::{
    controllers::interactive::data::fractal_config::FractalConfig,
    core::{
        data::{complex::Complex, complex_rect::ComplexRect, pixel_rect::PixelRect},
        fractals::julia::{
            algorithm::JuliaAlgorithm,
            colour_mapping::{factory::julia_colour_map_factory, kinds::JuliaColourMapKinds},
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
pub struct JuliaConfig {
    pub region: ComplexRect,
    pub max_iterations: u32,
    pub colour_map_kind: JuliaColourMapKinds,
}

impl Default for JuliaConfig {
    fn default() -> Self {
        Self {
            region: default_region(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            colour_map_kind: JuliaColourMapKinds::default(),
        }
    }
}

impl JuliaConfig {
    pub(crate) fn build_render_request(&self, pixel_rect: PixelRect) -> FractalConfig {
        let colour_map = julia_colour_map_factory(self.colour_map_kind, self.max_iterations);
        let algorithm = JuliaAlgorithm::new(pixel_rect, self.region, self.max_iterations)
            .expect("julia algorithm settings should be valid");

        FractalConfig::Julia {
            colour_map,
            algorithm,
        }
    }

    pub fn reset_view(&mut self) {
        self.region = default_region();
        self.max_iterations = DEFAULT_MAX_ITERATIONS;
    }
}
