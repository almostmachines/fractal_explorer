use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::mandelbrot::errors::mandelbrot::MandelbrotError;
use crate::core::util::pixel_to_complex_coords::{
    PixelToComplexCoordsError, pixel_to_complex_coords,
};

#[derive(Debug, PartialEq)]
pub struct MandelbrotAlgorithm {
    pub pixel_rect: PixelRect,
    complex_rect: ComplexRect,
    max_iterations: u32,
}

impl FractalAlgorithm for MandelbrotAlgorithm {
    type Success = u32;
    type Failure = PixelToComplexCoordsError;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
        let c = pixel_to_complex_coords(pixel, self.pixel_rect, self.complex_rect)?;
        Ok(self.iterate_point(c.real, c.imag))
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

        let mut c_real = complex_top_left.real + (x_start - top_left.x) as f64 * real_step;
        let c_imag = complex_top_left.imag + (y - top_left.y) as f64 * imag_step;

        for _x in x_start..=x_end {
            output.push(self.iterate_point(c_real, c_imag));
            c_real += real_step;
        }

        Ok(())
    }

    fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }
}

impl MandelbrotAlgorithm {
    #[inline]
    fn iterate_point(&self, c_real: f64, c_imag: f64) -> u32 {
        if Self::in_main_cardioid(c_real, c_imag) || Self::in_period2_bulb(c_real, c_imag) {
            return self.max_iterations;
        }

        let mut zr = 0.0f64;
        let mut zi = 0.0f64;
        let mut zr2 = 0.0f64;
        let mut zi2 = 0.0f64;

        let mut iteration = 1u32;
        while iteration <= self.max_iterations {
            let zr_next = zr2 - zi2 + c_real;
            let zi_next = (zr + zr) * zi + c_imag;
            zr = zr_next;
            zi = zi_next;
            zr2 = zr * zr;
            zi2 = zi * zi;

            if zr2 + zi2 > 4.0 {
                return iteration;
            }

            iteration += 1;
        }

        self.max_iterations
    }

    /// Returns true if c lies inside the main cardioid of the Mandelbrot set.
    fn in_main_cardioid(c_real: f64, c_imag: f64) -> bool {
        let q = (c_real - 0.25) * (c_real - 0.25) + c_imag * c_imag;
        q * (q + (c_real - 0.25)) <= 0.25 * c_imag * c_imag
    }

    /// Returns true if c lies inside the period-2 bulb (circle centred at -1+0i, radius 1/4).
    fn in_period2_bulb(c_real: f64, c_imag: f64) -> bool {
        (c_real + 1.0) * (c_real + 1.0) + c_imag * c_imag <= 0.0625
    }

    pub fn new(
        pixel_rect: PixelRect,
        complex_rect: ComplexRect,
        max_iterations: u32,
    ) -> Result<Self, MandelbrotError> {
        if max_iterations == 0 {
            return Err(MandelbrotError::ZeroMaxIterationsError);
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

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 256);

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

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 0);

        assert_eq!(
            algorithm,
            Err(MandelbrotError::ZeroMaxIterationsError {})
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

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 10).unwrap();
        let point = Point { x: 11, y: 0 };
        let result = algorithm.compute(point);

        assert_eq!(
            result,
            Err(PixelToComplexCoordsError::PointOutsideRect { point, pixel_rect })
        );
    }

    #[test]
    fn compute_returns_max_iterations_for_c_equal_zero() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: 100.0,
                imag: 100.0,
            },
        )
        .unwrap();

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
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: 3.0,
                imag: 3.0,
            },
        )
        .unwrap();

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, 2).unwrap();
        let iterations = algorithm.compute(Point { x: 3, y: 0 }).unwrap();

        assert_eq!(iterations, 1);
    }

    #[test]
    fn compute_returns_max_iterations_for_c_equal_negative_one() {
        // c = -1 + 0i is inside the set (it cycles: 0, -1, 0, -1, ...).
        // It should never escape -> returns max_iterations.
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 10 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.0,
                imag: 0.0,
            },
            Complex {
                real: 10.0,
                imag: 10.0,
            },
        )
        .unwrap();

        let max_iterations = 80;
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations).unwrap();

        let iterations = algorithm.compute(Point { x: 0, y: 0 }).unwrap();

        assert_eq!(iterations, max_iterations);
    }
}
