use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::julia::errors::julia::JuliaError;
use crate::core::util::pixel_to_complex_coords::{
    PixelToComplexCoordsError, pixel_to_complex_coords,
};
const PERIODICITY_EPSILON: f64 = 1e-12;
const JULIA_C_REAL: f64 = -0.7;
const JULIA_C_IMAG: f64 = 0.27;

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
        let z = pixel_to_complex_coords(pixel, self.pixel_rect, self.complex_rect)?;
        Ok(self.iterate_point(z.real, z.imag))
    }

    fn compute_row_segment_into(
        &self,
        y: i32,
        x_start: i32,
        x_end: i32,
        output: &mut Vec<Self::Success>,
    ) -> Result<(), Self::Failure> {
        if x_start > x_end {
            return Ok(());
        }

        let top_left = self.pixel_rect.top_left();
        let bottom_right = self.pixel_rect.bottom_right();
        let in_bounds = y >= top_left.y
            && y <= bottom_right.y
            && x_start >= top_left.x
            && x_end <= bottom_right.x;

        if !in_bounds {
            for x in x_start..=x_end {
                output.push(self.compute(Point { x, y })?);
            }
            return Ok(());
        }

        let real_step = self.complex_rect.width() / (self.pixel_rect.width() - 1) as f64;
        let imag_step = self.complex_rect.height() / (self.pixel_rect.height() - 1) as f64;
        let complex_top_left = self.complex_rect.top_left();

        let mut zr = complex_top_left.real + (x_start - top_left.x) as f64 * real_step;
        let zi = complex_top_left.imag + (y - top_left.y) as f64 * imag_step;

        for _x in x_start..=x_end {
            output.push(self.iterate_point(zr, zi));
            zr += real_step;
        }

        Ok(())
    }

    fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }
}

impl JuliaAlgorithm {
    #[inline]
    fn iterate_point(&self, mut zr: f64, mut zi: f64) -> u32 {
        let mut zr_ref = zr;
        let mut zi_ref = zi;
        let mut power = 1u32;
        let mut lambda = 0u32;

        let mut iteration = 1u32;
        while iteration <= self.max_iterations {
            let zr_next = zr * zr - zi * zi + JULIA_C_REAL;
            let zi_next = 2.0 * zr * zi + JULIA_C_IMAG;
            zr = zr_next;
            zi = zi_next;

            if zr * zr + zi * zi > 4.0 {
                return iteration;
            }

            let dr = zr - zr_ref;
            let di = zi - zi_ref;
            if dr * dr + di * di < PERIODICITY_EPSILON {
                return self.max_iterations;
            }

            lambda += 1;
            if lambda == power {
                zr_ref = zr;
                zi_ref = zi;
                power *= 2;
                lambda = 0;
            }

            iteration += 1;
        }

        self.max_iterations
    }

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
