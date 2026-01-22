use std::sync::Arc;
use crate::controllers::interactive::data::fractal_config::FractalConfig;
use crate::core::data::complex::Complex;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
use crate::core::fractals::mandelbrot::colour_mapping::factory::mandelbrot_colour_map_factory;
use crate::core::fractals::mandelbrot::colour_mapping::kinds::MandelbrotColourMapKinds;

const DEFAULT_MAX_ITERATIONS: u32 = 256;

fn default_region() -> ComplexRect {
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
    .expect("default mandelbrot region is valid")
}

pub struct UiState {
    pub region: ComplexRect,
    pub max_iterations: u32,
    pub colour_map_kind: MandelbrotColourMapKinds,
    last_submitted_request: Option<Arc<FractalConfig>>,
    pub latest_submitted_generation: u64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            region: default_region(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            colour_map_kind: MandelbrotColourMapKinds::default(),
            last_submitted_request: None,
            latest_submitted_generation: 0,
        }
    }
}

impl UiState {
    #[must_use]
    pub fn build_render_request(&self, pixel_rect: PixelRect) -> FractalConfig {
        let colour_map = mandelbrot_colour_map_factory(self.colour_map_kind, self.max_iterations);
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, self.region, self.max_iterations).unwrap();

        FractalConfig::Mandelbrot { colour_map, algorithm }
    }

    #[must_use]
    pub fn should_submit(&self, request: &FractalConfig) -> bool {
        self.last_submitted_request
            .as_ref()
            .is_none_or(|last| last.as_ref() != request)
    }

    pub fn record_submission(&mut self, request: Arc<FractalConfig>, generation: u64) {
        self.last_submitted_request = Some(request);
        self.latest_submitted_generation = generation;
    }

    pub fn reset_view(&mut self) {
        self.region = default_region();
        self.max_iterations = DEFAULT_MAX_ITERATIONS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::point::Point;

    fn create_pixel_rect(width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )
        .unwrap()
    }

    #[test]
    fn changing_colour_map_kind_triggers_should_submit() {
        let mut ui_state = UiState::default();
        let pixel_rect = create_pixel_rect(100, 100);

        // Submit initial request
        let request1 = ui_state.build_render_request(pixel_rect);
        ui_state.record_submission(Arc::new(request1), 1);

        // Same state should not need re-submission
        let same_request = ui_state.build_render_request(pixel_rect);
        assert!(!ui_state.should_submit(&same_request));

        // Change only colour_map_kind
        ui_state.colour_map_kind = MandelbrotColourMapKinds::BlueWhiteGradient;
        let changed_request = ui_state.build_render_request(pixel_rect);
        assert!(ui_state.should_submit(&changed_request));
    }
}
