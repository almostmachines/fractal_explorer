use std::sync::Arc;
use crate::controllers::interactive::data::fractal_config::FractalConfig;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::fractals::fractal_kinds::FractalKinds;
use crate::core::fractals::julia::julia_config::JuliaConfig;
use crate::core::fractals::mandelbrot::mandelbrot_config::MandelbrotConfig;

pub struct GuiAppState {
    pub selected_fractal: FractalKinds,
    pub mandelbrot: MandelbrotConfig,
    pub julia: JuliaConfig,
    last_submitted_request: Option<Arc<FractalConfig>>,
    pub latest_submitted_generation: u64,
    pub redraw_pending: bool,
}

impl Default for GuiAppState {
    fn default() -> Self {
        Self {
            selected_fractal: FractalKinds::default(),
            mandelbrot: MandelbrotConfig::default(),
            julia: JuliaConfig::default(),
            last_submitted_request: None,
            latest_submitted_generation: 0,
            redraw_pending: true,
        }
    }
}

impl GuiAppState {
    #[must_use]
    pub fn build_render_request(&self, pixel_rect: PixelRect) -> FractalConfig {
        match self.selected_fractal {
            FractalKinds::Mandelbrot => self.mandelbrot.build_render_request(pixel_rect),
            FractalKinds::Julia => self.julia.build_render_request(pixel_rect),
        }
    }

    #[must_use]
    pub fn active_region(&self) -> ComplexRect {
        match self.selected_fractal {
            FractalKinds::Mandelbrot => self.mandelbrot.region,
            FractalKinds::Julia => self.julia.region,
        }
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
        match self.selected_fractal {
            FractalKinds::Mandelbrot => self.mandelbrot.reset_view(),
            FractalKinds::Julia => self.julia.reset_view(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{data::point::Point, fractals::julia::colour_mapping::kinds::JuliaColourMapKinds};

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
        let mut ui_state = GuiAppState::default();
        let pixel_rect = create_pixel_rect(100, 100);

        // Submit initial request
        let request1 = ui_state.build_render_request(pixel_rect);
        ui_state.record_submission(Arc::new(request1), 1);

        // Same state should not need re-submission
        let same_request = ui_state.build_render_request(pixel_rect);
        assert!(!ui_state.should_submit(&same_request));

        // Change only colour_map_kind
        ui_state.julia.colour_map_kind = JuliaColourMapKinds::BlueWhiteGradient;
        let changed_request = ui_state.build_render_request(pixel_rect);
        assert!(ui_state.should_submit(&changed_request));
    }

    #[test]
    fn switching_selected_fractal_triggers_should_submit() {
        let mut ui_state = GuiAppState::default();
        let pixel_rect = create_pixel_rect(100, 100);

        let request1 = ui_state.build_render_request(pixel_rect);
        ui_state.record_submission(Arc::new(request1), 1);

        ui_state.selected_fractal = FractalKinds::Mandelbrot;
        let changed_request = ui_state.build_render_request(pixel_rect);

        assert!(ui_state.should_submit(&changed_request));
    }

    #[test]
    fn build_render_request_uses_selected_fractal_variant() {
        let mut ui_state = GuiAppState::default();
        let pixel_rect = create_pixel_rect(100, 100);

        assert!(matches!(
            ui_state.build_render_request(pixel_rect),
            FractalConfig::Julia { .. }
        ));

        ui_state.selected_fractal = FractalKinds::Mandelbrot;

        assert!(matches!(
            ui_state.build_render_request(pixel_rect),
            FractalConfig::Mandelbrot { .. }
        ));
    }

    #[test]
    fn switching_fractals_preserves_each_variant_settings() {
        let mut ui_state = GuiAppState::default();

        ui_state.julia.max_iterations = 111;
        ui_state.mandelbrot.max_iterations = 222;

        ui_state.selected_fractal = FractalKinds::Mandelbrot;
        assert_eq!(ui_state.mandelbrot.max_iterations, 222);

        ui_state.selected_fractal = FractalKinds::Julia;
        assert_eq!(ui_state.julia.max_iterations, 111);
    }
}
