use crate::core::{data::complex_rect::ComplexRect, fractals::julia::errors::julia::JuliaError};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct JuliaParams {
    region: ComplexRect,
    max_iterations: u32,
}

#[allow(dead_code)]
impl JuliaParams {
    pub fn new(region: ComplexRect, max_iterations: u32) -> Result<Self, JuliaError> {
        if max_iterations == 0 {
            return Err(JuliaError::ZeroMaxIterationsError);
        }

        Ok(
            Self {
                region,
                max_iterations,
            }
        )
    }

    pub fn display_name(&self) -> &str {
        "Julia"
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

    pub fn set_max_iterations(&mut self, max_iterations: u32) -> Result<(), JuliaError> {
        if max_iterations == 0 {
            return Err(JuliaError::ZeroMaxIterationsError);
        }

        self.max_iterations = max_iterations;
        Ok(())
    }
}
