use crate::core::{data::complex_rect::ComplexRect, fractals::mandelbrot::errors::mandelbrot::MandelbrotError};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MandelbrotParams {
    region: ComplexRect,
    max_iterations: u32,
}

#[allow(dead_code)]
impl MandelbrotParams {
    pub fn new(region: ComplexRect, max_iterations: u32) -> Result<Self, MandelbrotError> {
        if max_iterations == 0 {
            return Err(MandelbrotError::ZeroMaxIterationsError);
        }

        Ok(
            Self {
                region,
                max_iterations,
            }
        )
    }

    pub fn display_name(&self) -> &str {
        "Mandelbrot"
    }

    pub fn region(&self) -> ComplexRect {
        self.region
    }

    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    pub fn set_region(&mut self, region: ComplexRect) {
        self.region = region
    }

    pub fn set_max_iterations(&mut self, max_iterations: u32) -> Result<(), MandelbrotError> {
        if max_iterations == 0 {
            return Err(MandelbrotError::ZeroMaxIterationsError);
        }

        self.max_iterations = max_iterations;
        Ok(())
    }
}
