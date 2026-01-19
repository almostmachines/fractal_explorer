use crate::core::data::complex_rect::ComplexRect;

#[derive(Debug, Clone, PartialEq)]
pub enum FractalParams {
    Mandelbrot {
        region: ComplexRect,
        max_iterations: u32,
    },
}
