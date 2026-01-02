use rayon::prelude::*;

use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;

/// Generates fractal data in parallel using rayon's work-stealing scheduler.
///
/// This provides automatic load balancing and simpler API compared to manual threading.
#[allow(dead_code)]
pub fn generate_fractal_rayon<Alg>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
) -> Result<Vec<Alg::Success>, Alg::Failure>
where
    Alg: FractalAlgorithm + Sync,
    Alg::Success: Send,
    Alg::Failure: Send,
{
    let pixels: Vec<Point> = (pixel_rect.top_left().y..pixel_rect.bottom_right().y)
        .flat_map(|y| {
            (pixel_rect.top_left().x..pixel_rect.bottom_right().x).map(move |x| Point { x, y })
        })
        .collect();

    pixels
        .into_par_iter()
        .map(|pixel| algorithm.compute(pixel))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_fractal::generate_fractal::generate_fractal;
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

    #[derive(Debug)]
    struct StubFailureAlgorithm {}

    impl FractalAlgorithm for StubFailureAlgorithm {
        type Success = u64;
        type Failure = StubError;

        fn compute(&self, _: Point) -> Result<Self::Success, Self::Failure> {
            Err(StubError {})
        }
    }

    #[test]
    fn test_rayon_generates_same_results_as_sequential() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 8 }).unwrap();

        let sequential_results = generate_fractal(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }

    #[test]
    fn test_rayon_propagates_algorithm_failure() {
        let algorithm = StubFailureAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 4 }).unwrap();

        let result = generate_fractal_rayon(pixel_rect, &algorithm);

        assert!(result.is_err());
    }

    #[test]
    fn test_rayon_with_single_pixel() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 5, y: 5 }, Point { x: 6, y: 6 }).unwrap();

        let sequential_results = generate_fractal(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }

    #[test]
    fn test_rayon_with_large_rect() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let sequential_results = generate_fractal(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }
}
