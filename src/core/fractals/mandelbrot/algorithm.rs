use std::error::Error;
use std::fmt;
use std::ops::ControlFlow;
use crate::core::data::complex::Complex;
use crate::core::data::point::Point;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::util::pixel_to_complex_coords::{pixel_to_complex_coords, PixelToComplexCoordsError};

#[derive(Debug)]
pub struct MandelbrotAlgorithm {
    pixel_rect: PixelRect,
    complex_rect: ComplexRect,
    max_iterations: u32,
}

#[derive(Debug)]
pub enum MandelbrotAlgorithmConstructorError {
    ZeroMaxIterationsError,
}

impl fmt::Display for MandelbrotAlgorithmConstructorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaxIterationsError => {
                write!(f, "Maximum iterations must be greater than zero")
            }
        }
    }
}

impl Error for MandelbrotAlgorithmConstructorError {}

impl FractalAlgorithm for MandelbrotAlgorithm {
    type Success = u32;
    type Failure = PixelToComplexCoordsError;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
        let c = pixel_to_complex_coords(pixel, self.pixel_rect, self.complex_rect)?;
        let z = Complex { real: 0.0, imag: 0.0 };

        let iterations = (1..=self.max_iterations).try_fold(z, |z0, iteration| {
            if z0.magnitude_squared() > 4.0 {
                ControlFlow::Break(iteration - 1)
            } else {
                ControlFlow::Continue(z0 * z0 + c)
            }
        });

        Ok(
            match iterations {
                ControlFlow::Break(iteration) => iteration,
                ControlFlow::Continue(_) => self.max_iterations,
            }
        )
    }
}

impl MandelbrotAlgorithm {
    pub fn new(pixel_rect: PixelRect, complex_rect: ComplexRect, max_iterations: u32) -> Result<Self, MandelbrotAlgorithmConstructorError> {
        if max_iterations == 0 {
            return Err(MandelbrotAlgorithmConstructorError::ZeroMaxIterationsError)
        }

        Ok(Self { pixel_rect, complex_rect, max_iterations })
    }
}
