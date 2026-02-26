use std::error::Error;
use std::fmt;
use std::thread;

use crate::core::actions::generate_fractal::generate_fractal_serial::generate_fractal_serial;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::pixel_rect::{PixelRect, PixelRectError};
use crate::core::data::point::Point;
use crate::core::util::calculate_threads_for_pixel_rect_banding::calculate_threads_for_pixel_rect_banding;

#[derive(Debug)]
pub enum GenerateFractalParallelError<AlgFailure: Error> {
    Algorithm(AlgFailure),
    PixelRect(PixelRectError),
}

impl<AlgFailure: Error> fmt::Display for GenerateFractalParallelError<AlgFailure> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Algorithm(err) => write!(f, "fractal algorithm error: {}", err),
            Self::PixelRect(err) => write!(f, "pixel rect error: {}", err),
        }
    }
}

impl<AlgFailure: Error + 'static> Error for GenerateFractalParallelError<AlgFailure> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Algorithm(err) => Some(err),
            Self::PixelRect(err) => Some(err),
        }
    }
}

impl<AlgFailure: Error> From<PixelRectError> for GenerateFractalParallelError<AlgFailure> {
    fn from(err: PixelRectError) -> Self {
        Self::PixelRect(err)
    }
}

fn generate_pixel_rect_band(
    band_num: u32,
    band_height: u32,
    total_bands: u32,
    bounding_rect: PixelRect,
) -> Result<PixelRect, PixelRectError> {
    let band_top = (band_num * band_height) as i32;

    let band_bottom = if band_num == total_bands - 1 {
        (bounding_rect.height() - 1) as i32 // Last thread takes any remainder rows
    } else {
        (((band_num + 1) * band_height) - 1) as i32
    };

    let band_top_left = Point {
        x: bounding_rect.top_left().x,
        y: bounding_rect.top_left().y + band_top,
    };

    let band_bottom_right = Point {
        x: bounding_rect.bottom_right().x,
        y: bounding_rect.top_left().y + band_bottom,
    };

    PixelRect::new(band_top_left, band_bottom_right)
}

#[allow(dead_code)]
pub fn generate_fractal_parallel_scoped_threads<Alg: FractalAlgorithm + Send + Sync>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
) -> Result<Vec<Alg::Success>, GenerateFractalParallelError<Alg::Failure>>
where
    Alg::Success: Send,
    Alg::Failure: Send,
{
    let num_threads = calculate_threads_for_pixel_rect_banding(pixel_rect);
    let band_height = pixel_rect.height() / num_threads;

    let results = thread::scope(
        |scope| -> Result<Vec<Alg::Success>, GenerateFractalParallelError<Alg::Failure>> {
            let scoped_results = (0..num_threads)
            .map(|thread_idx| {
                scope.spawn(move || {
                    generate_fractal_serial(
                        generate_pixel_rect_band(thread_idx, band_height, num_threads, pixel_rect)?,
                        algorithm
                    )
                    .map_err(GenerateFractalParallelError::Algorithm)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("Thread panicked during fractal computation"))
            .collect::<
                Result<Vec<Vec<Alg::Success>>,
                GenerateFractalParallelError<Alg::Failure>>
            >()?
            .into_iter()
            .flatten()
            .collect::<Vec<Alg::Success>>();

            Ok(scoped_results)
        },
    )?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_fractal::generate_fractal_serial::generate_fractal_serial;
    use std::error::Error;

    #[derive(Debug, PartialEq)]
    struct StubError {}

    impl std::fmt::Display for StubError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "StubError")
        }
    }

    impl Error for StubError {}

    #[derive(Debug)]
    struct StubSuccessAlgorithm {}

    impl FractalAlgorithm for StubSuccessAlgorithm {
        type Success = u64;
        type Failure = StubError;

        fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
            Ok((pixel.x + pixel.y) as u64)
        }
    }

    #[test]
    fn test_parallel_generates_same_results_as_sequential() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 8 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_scoped_threads(pixel_rect, &algorithm).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_single_thread() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 5 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_scoped_threads(pixel_rect, &algorithm).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_uneven_row_distribution() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 7 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_scoped_threads(pixel_rect, &algorithm).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_more_threads_than_rows() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 2 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_scoped_threads(pixel_rect, &algorithm).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }
}
