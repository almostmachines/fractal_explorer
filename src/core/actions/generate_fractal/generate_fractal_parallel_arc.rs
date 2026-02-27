use std::sync::Arc;
use std::thread;

use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::util::calculate_threads_for_pixel_rect_banding::calculate_threads_for_pixel_rect_banding;

#[allow(dead_code)]
pub fn generate_fractal_parallel_arc<Alg>(
    pixel_rect: PixelRect,
    algorithm: Arc<Alg>,
) -> Result<Vec<Alg::Success>, Alg::Failure>
where
    Alg: FractalAlgorithm + Sync + Send + 'static,
    Alg::Success: Send,
    Alg::Failure: Send,
{
    let height = pixel_rect.height();
    let top_y = pixel_rect.top_left().y;
    let left_x = pixel_rect.top_left().x;
    let right_x = pixel_rect.bottom_right().x;
    let num_threads = calculate_threads_for_pixel_rect_banding(pixel_rect);
    let rows_per_thread = height / num_threads;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_idx| {
            let alg = Arc::clone(&algorithm);

            // Calculate row range for this thread
            let start_row = thread_idx * rows_per_thread;

            let end_row = if thread_idx == num_threads - 1 {
                height // Last thread takes any remainder rows
            } else {
                (thread_idx + 1) * rows_per_thread
            };

            thread::spawn(move || {
                let mut chunk_results = Vec::with_capacity(
                    (end_row - start_row) as usize * (right_x - left_x) as usize,
                );

                for row in start_row..end_row {
                    let y = top_y + row as i32;
                    for x in left_x..=right_x {
                        let pixel = Point { x, y };
                        let result = alg.compute(pixel)?;
                        chunk_results.push(result);
                    }
                }

                Ok(chunk_results)
            })
        })
        .collect();

    let total_pixels = pixel_rect.size() as usize;
    let mut results = Vec::with_capacity(total_pixels);

    for handle in handles {
        let chunk = handle
            .join()
            .expect("Thread panicked during fractal computation")?;
        results.extend(chunk);
    }

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

        fn pixel_rect(&self) -> PixelRect {
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 }).unwrap()
        }
    }

    #[test]
    fn test_parallel_generates_same_results_as_sequential() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 8 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_arc(pixel_rect, Arc::new(algorithm)).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_single_thread() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 5 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_arc(pixel_rect, Arc::new(algorithm)).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_uneven_row_distribution() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 7 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_arc(pixel_rect, Arc::new(algorithm)).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }

    #[test]
    fn test_parallel_with_more_threads_than_rows() {
        let algorithm = StubSuccessAlgorithm {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 2 }).unwrap();
        let sequential_results = generate_fractal_serial(pixel_rect, &algorithm).unwrap();
        let parallel_results = generate_fractal_parallel_arc(pixel_rect, Arc::new(algorithm)).unwrap();

        assert_eq!(parallel_results, sequential_results);
    }
}
