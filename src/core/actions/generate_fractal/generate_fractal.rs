use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;

#[allow(dead_code)]
pub fn generate_fractal<Alg: FractalAlgorithm>(pixel_rect: PixelRect, algorithm: &Alg) -> Result<Vec<Alg::Success>, Alg::Failure>
{
    (pixel_rect.top_left().y..pixel_rect.bottom_right().y)
        .flat_map(|y| {
            (pixel_rect.top_left().x..pixel_rect.bottom_right().x)
                .map(move |x| Point { x, y })
        })
        .map(|pixel| {
            let result = algorithm.compute(pixel)?;
            Ok(result)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;

    #[derive(Debug, PartialEq)]
    struct StubError {}

    impl std::fmt::Display for StubError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "StubError")
        }
    }

    impl Error for StubError {}

    #[derive(Debug, PartialEq)]
    struct StubSuccessAlgorithm {}

    impl FractalAlgorithm for StubSuccessAlgorithm {
        type Success = u64;
        type Failure = StubError;

        fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
            Ok((pixel.x + pixel.y) as u64)
        }
    }

    #[derive(Debug, PartialEq)]
    struct StubFailureAlgorithm {}

    impl FractalAlgorithm for StubFailureAlgorithm {
        type Success = u64;
        type Failure = StubError;

        fn compute(&self, _: Point) -> Result<Self::Success, Self::Failure> {
            Err(StubError {})
        }
    }

    #[test]
    fn test_generates_correct_results() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x:0, y:0 }, Point { x:3, y:4 }).unwrap();
        let expected_results: Vec<u64> = vec![0, 1, 2, 1, 2, 3, 2, 3, 4, 3, 4, 5];
        let results = generate_fractal(pixel_rect, &algorithm).unwrap();

        assert_eq!(results, expected_results);
    }

    #[test]
    fn test_propagates_algorithm_failure() {
        let algorithm = StubFailureAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x:0, y:0 }, Point { x:3, y:4 }).unwrap();
        let results = generate_fractal(pixel_rect, &algorithm);

        assert_eq!(results, Err(StubError {}));
    }
}
