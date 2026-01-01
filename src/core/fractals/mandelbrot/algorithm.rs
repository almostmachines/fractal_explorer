use std::error::Error;
use std::fmt;
use std::ops::ControlFlow;
use crate::core::data::complex::Complex;
use crate::core::data::point::Point;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::util::pixel_to_complex_coords::{pixel_to_complex_coords, PixelToComplexCoordsError};

#[derive(Debug, PartialEq)]
pub struct MandelbrotAlgorithm {
    pixel_rect: PixelRect,
    complex_rect: ComplexRect,
    max_iterations: u32,
}

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::complex::Complex;
    use crate::core::data::point::Point;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::complex_rect::ComplexRect;
    use crate::core::util::pixel_to_complex_coords::PixelToComplexCoordsError;

    #[test]
    fn test_valid_constructor() {
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0},
            Point { x: 700, y: 400}
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: -2.5, imag: -1.0 },
            Complex { real: 1.0, imag: 1.0 }
        ).unwrap();

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 256);
        
        assert!(algorithm.is_ok());
    }

    #[test]
    fn test_max_iterations_must_be_greater_than_zero() {
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0},
            Point { x: 700, y: 400}
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: -2.5, imag: -1.0 },
            Complex { real: 1.0, imag: 1.0 }
        ).unwrap();

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 0);
        
        assert_eq!(algorithm, Err(MandelbrotAlgorithmConstructorError::ZeroMaxIterationsError {}));
    }

    #[test]
    fn compute_returns_error_for_pixel_outside_pixel_rect() {
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point { x: 10, y: 10 }
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: -2.5, imag: -1.0 },
            Complex { real: 1.0, imag: 1.0 },
        ).unwrap();

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 10).unwrap();
        let point = Point { x: 11, y: 0 };
        let result = algorithm.compute(point);

        assert_eq!(result, Err(PixelToComplexCoordsError::PointOutsideRect { point, pixel_rect }));
    }

    #[test]
    fn compute_returns_max_iterations_for_c_equal_zero() {
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point { x: 100, y: 100 }
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: 0.0, imag: 0.0 },
            Complex { real: 100.0, imag: 100.0 },
        ).unwrap();

        let max_iterations = 25;
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations).unwrap();
        let iterations = algorithm.compute(Point { x: 0, y: 0 }).unwrap();

        assert_eq!(iterations, max_iterations);
    }

    #[test]
    fn compute_escapes_immediately_for_c_equal_three() {
        // Map pixel (0,0) -> complex (3,0)
        // z0 = 0
        // iteration 1: z = 0^2 + 3 = 3 (|z|^2 = 9)
        // iteration 2: check sees |z|^2 > 4, breaks with iteration-1 => 1
        let pixel_rect =
            PixelRect::new(
                Point { x: 0, y: 0 },
                Point { x: 3, y: 3 }
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: 0.0, imag: 0.0 },
            Complex { real: 3.0, imag: 3.0 },
        ).unwrap();

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 2).unwrap();
        let iterations = algorithm.compute(Point { x: 3, y: 0 }).unwrap();

        assert_eq!(iterations, 1);
    }

    #[test]
    fn compute_returns_max_iterations_for_c_equal_negative_one() {
        // c = -1 + 0i is inside the set (it cycles: 0, -1, 0, -1, ...).
        // It should never escape -> returns max_iterations.
        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point { x: 10, y: 10 }
        ).unwrap();

        let complex_rect = ComplexRect::new(
            Complex { real: -1.0, imag: 0.0 },
            Complex { real: 10.0, imag: 10.0 },
        ).unwrap();

        let max_iterations = 80;
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations).unwrap();

        let iterations = algorithm.compute(Point { x: 0, y: 0 }).unwrap();

        assert_eq!(iterations, max_iterations);
    }
}
