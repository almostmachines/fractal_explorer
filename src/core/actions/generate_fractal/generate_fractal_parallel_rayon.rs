use rayon::prelude::*;

use crate::core::actions::cancellation::{
    CancelToken, Cancelled, NeverCancel, CANCEL_CHECK_INTERVAL_PIXELS,
};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;

/// Error type for cancelable fractal generation.
///
/// Distinguishes between algorithm failures and cancellation, allowing callers
/// to handle each case appropriately (e.g., not displaying cancellation as errors).
#[derive(Debug)]
pub enum GenerateFractalError<E> {
    /// The operation was cancelled before completion.
    Cancelled(Cancelled),
    /// The fractal algorithm reported a failure.
    Algorithm(E),
}

impl<E: std::fmt::Display> std::fmt::Display for GenerateFractalError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateFractalError::Cancelled(c) => write!(f, "{}", c),
            GenerateFractalError::Algorithm(e) => write!(f, "algorithm error: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for GenerateFractalError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GenerateFractalError::Cancelled(c) => Some(c),
            GenerateFractalError::Algorithm(e) => Some(e),
        }
    }
}

/// Generates fractal data in parallel using rayon's work-stealing scheduler.
///
/// This provides automatic load balancing and simpler API compared to manual threading.
/// For cancel-aware generation, use [`generate_fractal_parallel_rayon_cancelable`].
#[allow(dead_code)]
pub fn generate_fractal_parallel_rayon<Alg>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
) -> Result<Vec<Alg::Success>, Alg::Failure>
where
    Alg: FractalAlgorithm + Sync,
    Alg::Success: Send,
    Alg::Failure: Send,
{
    // Delegate to the cancel-aware implementation with NeverCancel
    generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, algorithm, &NeverCancel)
        .map_err(|e| match e {
            GenerateFractalError::Algorithm(alg_err) => alg_err,
            GenerateFractalError::Cancelled(_) => {
                // NeverCancel never cancels, so this branch is unreachable
                unreachable!("NeverCancel token should never signal cancellation")
            }
        })
}

/// Generates fractal data in parallel with cancellation support.
///
/// Like [`generate_fractal_parallel_rayon`], but accepts a cancellation token
/// that can abort the computation early. Checks for cancellation at the start
/// of each row and periodically within rows.
///
/// Returns [`GenerateFractalError::Cancelled`] if cancellation was requested,
/// which should be handled as expected control flow (not an error to display).
#[allow(dead_code)]
pub fn generate_fractal_parallel_rayon_cancelable<Alg, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    cancel: &C,
) -> Result<Vec<Alg::Success>, GenerateFractalError<Alg::Failure>>
where
    Alg: FractalAlgorithm + Sync,
    Alg::Success: Send,
    Alg::Failure: Send,
    C: CancelToken,
{
    generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, algorithm, cancel)
}

/// Internal cancel-aware fractal generation implementation.
///
/// Processes rows in parallel, checking for cancellation at the start of each
/// row and every [`CANCEL_CHECK_INTERVAL_PIXELS`] pixels within a row. Uses
/// rayon's try combinators to abort promptly when cancellation is detected.
///
/// Returns row-major ordered results, matching the output format of
/// [`generate_fractal_parallel_rayon`].
#[allow(dead_code)]
pub(crate) fn generate_fractal_parallel_rayon_cancelable_impl<Alg, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    cancel: &C,
) -> Result<Vec<Alg::Success>, GenerateFractalError<Alg::Failure>>
where
    Alg: FractalAlgorithm + Sync,
    Alg::Success: Send,
    Alg::Failure: Send,
    C: CancelToken,
{
    let y_range: Vec<i32> = (pixel_rect.top_left().y..=pixel_rect.bottom_right().y).collect();
    let x_start = pixel_rect.top_left().x;
    let x_end = pixel_rect.bottom_right().x;
    let row_width = (x_end - x_start + 1) as usize;

    // Process rows in parallel, each row checks cancellation at start and periodically
    let rows: Result<Vec<Vec<Alg::Success>>, GenerateFractalError<Alg::Failure>> = y_range
        .into_par_iter()
        .map(|y| {
            let mut row = Vec::with_capacity(row_width);

            for (i, x) in (x_start..=x_end).enumerate() {
                // Check cancellation at row start (i == 0) and every N pixels
                if i % CANCEL_CHECK_INTERVAL_PIXELS == 0 && cancel.is_cancelled() {
                    return Err(GenerateFractalError::Cancelled(Cancelled));
                }

                let result = algorithm
                    .compute(Point { x, y })
                    .map_err(GenerateFractalError::Algorithm)?;
                row.push(result);
            }

            Ok(row)
        })
        .collect();

    // Flatten rows into row-major order
    rows.map(|r| r.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_fractal::generate_fractal_serial::generate_fractal_serial;
    use std::error::Error;
    use std::sync::atomic::{AtomicBool, Ordering};

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

        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }

    #[test]
    fn test_rayon_propagates_algorithm_failure() {
        let algorithm = StubFailureAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 4 }).unwrap();

        let result = generate_fractal_parallel_rayon(pixel_rect, &algorithm);

        assert!(result.is_err());
    }

    #[test]
    fn test_rayon_with_smallest_dimensions() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 5, y: 5 }, Point { x: 6, y: 6 }).unwrap();

        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }

    #[test]
    fn test_rayon_with_large_rect() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let rayon_results = generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();

        assert_eq!(rayon_results, sequential_results);
    }

    // Tests for cancelable implementation

    #[test]
    fn test_cancelable_produces_same_results_when_not_cancelled() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 8 }).unwrap();

        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let cancelable_results =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &NeverCancel)
                .unwrap();

        assert_eq!(cancelable_results, sequential_results);
    }

    #[test]
    fn test_cancelable_returns_cancelled_when_token_is_cancelled() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 8 }).unwrap();
        let cancelled = AtomicBool::new(true);
        let cancel_token = || cancelled.load(Ordering::Relaxed);

        let result =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &cancel_token);

        assert!(matches!(result, Err(GenerateFractalError::Cancelled(_))));
    }

    #[test]
    fn test_cancelable_propagates_algorithm_failure() {
        let algorithm = StubFailureAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 4 }).unwrap();

        let result =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &NeverCancel);

        assert!(matches!(result, Err(GenerateFractalError::Algorithm(_))));
    }

    #[test]
    fn test_cancelable_with_large_rect() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let cancelable_results =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &NeverCancel)
                .unwrap();

        assert_eq!(cancelable_results, sequential_results);
    }

    #[test]
    fn test_generate_fractal_error_displays_cancelled() {
        let err: GenerateFractalError<StubError> = GenerateFractalError::Cancelled(Cancelled);
        assert_eq!(format!("{}", err), "operation cancelled");
    }

    #[test]
    fn test_generate_fractal_error_displays_algorithm_error() {
        let err: GenerateFractalError<StubError> =
            GenerateFractalError::Algorithm(StubError {});
        assert_eq!(format!("{}", err), "algorithm error: StubError");
    }

    #[test]
    fn test_cancelable_cancels_after_k_polls() {
        use std::sync::atomic::AtomicUsize;

        let algorithm = StubSuccessAlgorithm {};
        // Use a small rect that will require multiple polls
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 5 }).unwrap();

        let poll_count = AtomicUsize::new(0);
        let cancel_after = 3; // Cancel after 3 polls
        let cancel_token = || {
            let count = poll_count.fetch_add(1, Ordering::Relaxed);
            count >= cancel_after
        };

        let result =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &cancel_token);

        // Should have been cancelled
        assert!(matches!(result, Err(GenerateFractalError::Cancelled(_))));
        // Token was polled at least cancel_after times
        assert!(poll_count.load(Ordering::Relaxed) >= cancel_after);
    }

    #[test]
    fn test_cancellation_polled_at_row_start() {
        use std::sync::atomic::AtomicUsize;

        let algorithm = StubSuccessAlgorithm {};
        // Narrow rect: 2 pixels wide, 5 rows tall
        // Each row should check cancellation at start (x == 0)
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 1, y: 4 }).unwrap();

        let poll_count = AtomicUsize::new(0);
        let cancel_token = || {
            poll_count.fetch_add(1, Ordering::Relaxed);
            false // Never cancel
        };

        let result =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &cancel_token);

        // Should succeed
        assert!(result.is_ok());
        // With 5 rows, should poll at least 5 times (once per row start)
        let polls = poll_count.load(Ordering::Relaxed);
        assert!(polls >= 5, "Expected at least 5 polls for 5 rows, got {}", polls);
    }

    #[test]
    fn test_cancellation_polled_multiple_times_on_wide_rows() {
        use std::sync::atomic::AtomicUsize;

        let algorithm = StubSuccessAlgorithm {};
        // Wide rect: 3000 pixels wide (well over CANCEL_CHECK_INTERVAL_PIXELS), 2 rows
        // Each row should poll at least 3 times: at 0, 1024, 2048
        // Min size is 2x2, so we use height 2
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2999, y: 1 }).unwrap();

        let poll_count = AtomicUsize::new(0);
        let cancel_token = || {
            poll_count.fetch_add(1, Ordering::Relaxed);
            false // Never cancel
        };

        let result =
            generate_fractal_parallel_rayon_cancelable_impl(pixel_rect, &algorithm, &cancel_token);

        // Should succeed
        assert!(result.is_ok());
        // With 3000 pixels per row, interval of 1024, and 2 rows, should poll at least 6 times
        let polls = poll_count.load(Ordering::Relaxed);
        assert!(polls >= 6, "Expected at least 6 polls for 2 wide rows, got {}", polls);
    }
}
