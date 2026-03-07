use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::julia::errors::julia::JuliaError;
use crate::core::util::pixel_to_complex_coords::{
    PixelToComplexCoordsError, pixel_to_complex_coords,
};
#[cfg(target_arch = "x86")]
use std::arch::x86::{
    _CMP_GT_OQ, _mm256_add_pd, _mm256_cmp_pd, _mm256_loadu_pd, _mm256_movemask_pd,
    _mm256_mul_pd, _mm256_set1_pd, _mm256_sub_pd,
};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    _CMP_GT_OQ, _mm256_add_pd, _mm256_cmp_pd, _mm256_loadu_pd, _mm256_movemask_pd,
    _mm256_mul_pd, _mm256_set1_pd, _mm256_sub_pd,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const AVX_LANES: usize = 4;

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

        let zr = complex_top_left.real + (x_start - top_left.x) as f64 * real_step;
        let zi = complex_top_left.imag + (y - top_left.y) as f64 * imag_step;

        let point_count = (x_end - x_start + 1) as usize;
        output.reserve(point_count);

        if self.append_row_segment_avx(zr, zi, real_step, point_count, output) {
            return Ok(());
        }

        self.append_row_segment_scalar(zr, zi, real_step, point_count, output);

        Ok(())
    }

    fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }
}

impl JuliaAlgorithm {
    #[inline]
    fn append_row_segment_scalar(
        &self,
        mut zr: f64,
        zi: f64,
        real_step: f64,
        point_count: usize,
        output: &mut Vec<u32>,
    ) {
        for _ in 0..point_count {
            output.push(self.iterate_point(zr, zi));
            zr += real_step;
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn append_row_segment_avx(
        &self,
        mut zr: f64,
        zi: f64,
        real_step: f64,
        point_count: usize,
        output: &mut Vec<u32>,
    ) -> bool {
        if !is_x86_feature_detected!("avx") {
            return false;
        }

        let simd_points = point_count / AVX_LANES * AVX_LANES;
        let chunk_step = real_step * AVX_LANES as f64;

        for _ in 0..(simd_points / AVX_LANES) {
            let lane_reals = [zr, zr + real_step, zr + real_step * 2.0, zr + real_step * 3.0];
            let lane_iters = unsafe { self.iterate_four_points_avx(lane_reals, zi) };
            output.extend_from_slice(&lane_iters);
            zr += chunk_step;
        }

        self.append_row_segment_scalar(zr, zi, real_step, point_count - simd_points, output);

        true
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    fn append_row_segment_avx(
        &self,
        _zr: f64,
        _zi: f64,
        _real_step: f64,
        _point_count: usize,
        _output: &mut Vec<u32>,
    ) -> bool {
        false
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx")]
    unsafe fn iterate_four_points_avx(
        &self,
        lane_reals: [f64; AVX_LANES],
        zi: f64,
    ) -> [u32; AVX_LANES] {
        let mut results = [self.max_iterations; AVX_LANES];
        let mut active_mask = (1u8 << AVX_LANES) - 1;

        let julia_c_real = _mm256_set1_pd(JULIA_C_REAL);
        let julia_c_imag = _mm256_set1_pd(JULIA_C_IMAG);
        let escape_radius_sq = _mm256_set1_pd(4.0);
        let mut zr = unsafe { _mm256_loadu_pd(lane_reals.as_ptr()) };
        let mut zi = _mm256_set1_pd(zi);
        let mut zr2 = _mm256_mul_pd(zr, zr);
        let mut zi2 = _mm256_mul_pd(zi, zi);

        for iteration in 1..=self.max_iterations {
            let zr_next = _mm256_add_pd(_mm256_sub_pd(zr2, zi2), julia_c_real);
            let zi_next = _mm256_add_pd(_mm256_mul_pd(_mm256_add_pd(zr, zr), zi), julia_c_imag);
            zr = zr_next;
            zi = zi_next;
            zr2 = _mm256_mul_pd(zr, zr);
            zi2 = _mm256_mul_pd(zi, zi);

            let magnitude_sq = _mm256_add_pd(zr2, zi2);
            let escaped_mask =
                _mm256_movemask_pd(_mm256_cmp_pd(magnitude_sq, escape_radius_sq, _CMP_GT_OQ))
                    as u8;
            let newly_escaped = escaped_mask & active_mask;

            if newly_escaped == 0 {
                continue;
            }

            for lane in 0..AVX_LANES {
                if (newly_escaped & (1 << lane)) != 0 {
                    results[lane] = iteration;
                }
            }

            active_mask &= !escaped_mask;
            if active_mask == 0 {
                break;
            }
        }

        results
    }

    #[inline]
    fn iterate_point(&self, mut zr: f64, mut zi: f64) -> u32 {
        let mut zr2 = zr * zr;
        let mut zi2 = zi * zi;

        let mut iteration = 1u32;
        while iteration <= self.max_iterations {
            let zr_next = zr2 - zi2 + JULIA_C_REAL;
            let zi_next = (zr + zr) * zi + JULIA_C_IMAG;
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

    #[test]
    fn compute_row_segment_matches_scalar_reference() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 31, y: 11 }).unwrap();
        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.25,
                imag: -0.75,
            },
            Complex {
                real: 1.25,
                imag: 0.75,
            },
        )
        .unwrap();
        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 512).unwrap();

        let y = 6;
        let x_start = 3;
        let x_end = 29;
        let expected: Vec<u32> = (x_start..=x_end)
            .map(|x| algorithm.compute(Point { x, y }).unwrap())
            .collect();

        let mut actual = Vec::new();
        algorithm
            .compute_row_segment_into(y, x_start, x_end, &mut actual)
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_row_segment_appends_results_after_existing_prefix() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 15, y: 7 }).unwrap();
        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.5,
                imag: -1.0,
            },
            Complex {
                real: 1.5,
                imag: 1.0,
            },
        )
        .unwrap();
        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 128).unwrap();

        let y = 4;
        let x_start = 2;
        let x_end = 12;
        let mut actual = vec![999];
        algorithm
            .compute_row_segment_into(y, x_start, x_end, &mut actual)
            .unwrap();

        let mut expected = vec![999];
        expected.extend((x_start..=x_end).map(|x| algorithm.compute(Point { x, y }).unwrap()));

        assert_eq!(actual, expected);
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn avx_row_kernel_matches_scalar_reference_when_available() {
        if !is_x86_feature_detected!("avx") {
            return;
        }

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 19, y: 9 }).unwrap();
        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.25,
                imag: -0.75,
            },
            Complex {
                real: 1.25,
                imag: 0.75,
            },
        )
        .unwrap();
        let algorithm = JuliaAlgorithm::new(pixel_rect, complex_rect, 256).unwrap();
        let top_left = pixel_rect.top_left();
        let y = 6;
        let x_start = 1;
        let x_end = 13;
        let point_count = (x_end - x_start + 1) as usize;
        let real_step = complex_rect.width() / (pixel_rect.width() - 1) as f64;
        let imag_step = complex_rect.height() / (pixel_rect.height() - 1) as f64;
        let complex_top_left = complex_rect.top_left();
        let zr = complex_top_left.real + (x_start - top_left.x) as f64 * real_step;
        let zi = complex_top_left.imag + (y - top_left.y) as f64 * imag_step;

        let mut actual = Vec::new();
        assert!(algorithm.append_row_segment_avx(zr, zi, real_step, point_count, &mut actual));

        let expected: Vec<u32> = (x_start..=x_end)
            .map(|x| algorithm.compute(Point { x, y }).unwrap())
            .collect();

        assert_eq!(actual, expected);
    }
}
