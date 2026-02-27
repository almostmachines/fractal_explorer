use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::complex::Complex;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::julia::errors::julia::JuliaError;
use crate::core::util::pixel_to_complex_coords::{
    PixelToComplexCoordsError, pixel_to_complex_coords,
};
use std::ops::ControlFlow;

#[derive(Debug, PartialEq)]
pub struct JuliaAlgorithm {
    pub pixel_rect: PixelRect,
    complex_rect: ComplexRect,
    max_iterations: u32,
}

impl FractalAlgorithm for JuliaAlgorithm {
    type Success = u32;
    type Failure = PixelToComplexCoordsError;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
        let c = Complex {
            real: -0.7,
            imag: 0.27,
        };

        let z = pixel_to_complex_coords(pixel, self.pixel_rect, self.complex_rect)?;

        let iterations = (1..=self.max_iterations).try_fold(z, |z0, iteration| {
            if z0.magnitude_squared() > 4.0 {
                ControlFlow::Break(iteration - 1)
            } else {
                ControlFlow::Continue(z0 * z0 + c)
            }
        });

        Ok(match iterations {
            ControlFlow::Break(iteration) => iteration,
            ControlFlow::Continue(_) => self.max_iterations,
        })
    }

    fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }
}

impl JuliaAlgorithm {
    pub fn new(
        pixel_rect: PixelRect,
        complex_rect: ComplexRect,
        max_iterations: u32,
    ) -> Result<Self, JuliaError> {
        if max_iterations == 0 {
            return Err(JuliaError::ZeroMaxIterationsError);
        }

        Ok(Self {
            pixel_rect,
            complex_rect,
            max_iterations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::complex::Complex;
    use crate::core::data::complex_rect::ComplexRect;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;
    use crate::core::util::pixel_to_complex_coords::PixelToComplexCoordsError;

    #[test]
    fn test_valid_constructor() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 700, y: 400 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 256);

        assert!(algorithm.is_ok());
    }

    #[test]
    fn test_max_iterations_must_be_greater_than_zero() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 700, y: 400 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 0);

        assert_eq!(
            algorithm,
            Err(JuliaError::ZeroMaxIterationsError {})
        );
    }

    #[test]
    fn compute_returns_error_for_pixel_outside_pixel_rect() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 10 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 10).unwrap();
        let point = Point { x: 11, y: 0 };
        let result = algorithm.compute(point);

        assert_eq!(
            result,
            Err(PixelToComplexCoordsError::PointOutsideRect { point, pixel_rect })
        );
    }
}
