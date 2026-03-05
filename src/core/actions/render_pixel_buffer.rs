use rayon::prelude::*;

use crate::core::actions::cancellation::{
    CancelToken, Cancelled, NeverCancel, CANCEL_CHECK_INTERVAL_PIXELS,
};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::actions::generate_pixel_buffer::ports::colour_map::{ColourMap, ColourMapError};
use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferData, PixelBufferError};
use crate::core::data::pixel_rect::PixelRect;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum RenderPixelBufferError<AlgErr> {
    Algorithm(AlgErr),
    ColourMap(ColourMapError),
    PixelBuffer(PixelBufferError),
}

impl<E: fmt::Display> fmt::Display for RenderPixelBufferError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Algorithm(e) => write!(f, "algorithm error: {}", e),
            Self::ColourMap(e) => write!(f, "colour map error: {}", e),
            Self::PixelBuffer(e) => write!(f, "pixel buffer error: {}", e),
        }
    }
}

impl<E: Error + 'static> Error for RenderPixelBufferError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Algorithm(e) => Some(e),
            Self::ColourMap(e) => Some(e.as_ref()),
            Self::PixelBuffer(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum RenderPixelBufferCancelableError<AlgErr> {
    Cancelled(Cancelled),
    Algorithm(AlgErr),
    ColourMap(ColourMapError),
    PixelBuffer(PixelBufferError),
}

impl<E: fmt::Display> fmt::Display for RenderPixelBufferCancelableError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled(c) => write!(f, "{}", c),
            Self::Algorithm(e) => write!(f, "algorithm error: {}", e),
            Self::ColourMap(e) => write!(f, "colour map error: {}", e),
            Self::PixelBuffer(e) => write!(f, "pixel buffer error: {}", e),
        }
    }
}

impl<E: Error + 'static> Error for RenderPixelBufferCancelableError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled(c) => Some(c),
            Self::Algorithm(e) => Some(e),
            Self::ColourMap(e) => Some(e.as_ref()),
            Self::PixelBuffer(e) => Some(e),
        }
    }
}

pub fn render_pixel_buffer_parallel_rayon<Alg, CMap>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
) -> Result<PixelBuffer, RenderPixelBufferError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
{
    render_pixel_buffer_parallel_rayon_cancelable_impl(pixel_rect, algorithm, colour_map, &NeverCancel)
        .map_err(|e| match e {
            RenderPixelBufferCancelableError::Cancelled(_) => {
                unreachable!("NeverCancel token should never signal cancellation")
            }
            RenderPixelBufferCancelableError::Algorithm(e) => RenderPixelBufferError::Algorithm(e),
            RenderPixelBufferCancelableError::ColourMap(e) => RenderPixelBufferError::ColourMap(e),
            RenderPixelBufferCancelableError::PixelBuffer(e) => {
                RenderPixelBufferError::PixelBuffer(e)
            }
        })
}

pub fn render_pixel_buffer_parallel_rayon_cancelable<Alg, CMap, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
    cancel: &C,
) -> Result<PixelBuffer, RenderPixelBufferCancelableError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
    C: CancelToken,
{
    render_pixel_buffer_parallel_rayon_cancelable_impl(pixel_rect, algorithm, colour_map, cancel)
}

fn render_pixel_buffer_parallel_rayon_cancelable_impl<Alg, CMap, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
    cancel: &C,
) -> Result<PixelBuffer, RenderPixelBufferCancelableError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
    C: CancelToken,
{
    let width = pixel_rect.width() as usize;
    let row_bytes = width * PixelBuffer::BYTES_PER_PIXEL;
    let x_start = pixel_rect.top_left().x;
    let x_end = pixel_rect.bottom_right().x;
    let top_y = pixel_rect.top_left().y;

    let mut buffer: PixelBufferData =
        vec![0u8; width * pixel_rect.height() as usize * PixelBuffer::BYTES_PER_PIXEL];

    buffer
        .par_chunks_mut(row_bytes)
        .enumerate()
        .try_for_each(
            |(row_idx, row)| -> Result<(), RenderPixelBufferCancelableError<Alg::Failure>> {
                if cancel.is_cancelled() {
                    return Err(RenderPixelBufferCancelableError::Cancelled(Cancelled));
                }

                let y = top_y + row_idx as i32;
                let mut chunk_start = x_start;
                let mut iters = Vec::with_capacity(CANCEL_CHECK_INTERVAL_PIXELS);

                while chunk_start <= x_end {
                    if cancel.is_cancelled() {
                        return Err(RenderPixelBufferCancelableError::Cancelled(Cancelled));
                    }

                    let chunk_end = chunk_start
                        .saturating_add(CANCEL_CHECK_INTERVAL_PIXELS as i32 - 1)
                        .min(x_end);

                    iters.clear();
                    algorithm
                        .compute_row_segment_into(y, chunk_start, chunk_end, &mut iters)
                        .map_err(RenderPixelBufferCancelableError::Algorithm)?;

                    for (offset, iter_val) in iters.iter().enumerate() {
                        let c = colour_map
                            .map(*iter_val)
                            .map_err(RenderPixelBufferCancelableError::ColourMap)?;
                        let base = ((chunk_start - x_start) as usize + offset)
                            * PixelBuffer::BYTES_PER_PIXEL;
                        row[base] = c.r;
                        row[base + 1] = c.g;
                        row[base + 2] = c.b;
                        row[base + 3] = PixelBuffer::ALPHA_OPAQUE;
                    }

                    chunk_start = chunk_end + 1;
                }
                Ok(())
            },
        )?;

    PixelBuffer::from_data_opaque(pixel_rect, buffer)
        .map_err(RenderPixelBufferCancelableError::PixelBuffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
    use crate::core::data::colour::Colour;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug, PartialEq)]
    struct StubAlgError;

    impl fmt::Display for StubAlgError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "StubAlgError")
        }
    }

    impl Error for StubAlgError {}

    #[derive(Debug)]
    struct StubAlgorithm;

    impl FractalAlgorithm for StubAlgorithm {
        type Success = u32;
        type Failure = StubAlgError;

        fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
            Ok((pixel.x + pixel.y) as u32)
        }

        fn pixel_rect(&self) -> PixelRect {
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 }).unwrap()
        }
    }

    #[derive(Debug)]
    struct FailingAlgorithm;

    impl FractalAlgorithm for FailingAlgorithm {
        type Success = u32;
        type Failure = StubAlgError;

        fn compute(&self, _: Point) -> Result<Self::Success, Self::Failure> {
            Err(StubAlgError)
        }

        fn pixel_rect(&self) -> PixelRect {
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 }).unwrap()
        }
    }

    #[derive(Debug)]
    struct StubColourMap;

    impl ColourMap<u32> for StubColourMap {
        fn map(&self, value: u32) -> Result<Colour, ColourMapError> {
            let v = (value & 0xFF) as u8;
            Ok(Colour { r: v, g: v, b: v })
        }

        fn display_name(&self) -> &str {
            "Stub"
        }
    }

    #[derive(Debug)]
    struct FailingColourMap;

    impl ColourMap<u32> for FailingColourMap {
        fn map(&self, _: u32) -> Result<Colour, ColourMapError> {
            Err("StubColourMapError".into())
        }

        fn display_name(&self) -> &str {
            "Failing"
        }
    }

    #[test]
    fn produces_correct_pixel_buffer() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let result = render_pixel_buffer_parallel_rayon(pixel_rect, &StubAlgorithm, &StubColourMap);
        let pb = result.unwrap();

        assert_eq!(pb.pixel_rect(), pixel_rect);

        // Row 0: (0,0)=0, (1,0)=1, (2,0)=2
        // Row 1: (0,1)=1, (1,1)=2, (2,1)=3
        let expected: Vec<u8> = vec![
            0, 0, 0, 255, 1, 1, 1, 255, 2, 2, 2, 255, 1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255,
        ];
        assert_eq!(pb.buffer(), &expected);
    }

    #[test]
    fn propagates_algorithm_error() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let result =
            render_pixel_buffer_parallel_rayon(pixel_rect, &FailingAlgorithm, &StubColourMap);
        assert!(matches!(result, Err(RenderPixelBufferError::Algorithm(_))));
    }

    #[test]
    fn propagates_colour_map_error() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let result =
            render_pixel_buffer_parallel_rayon(pixel_rect, &StubAlgorithm, &FailingColourMap);
        assert!(matches!(result, Err(RenderPixelBufferError::ColourMap(_))));
    }

    #[test]
    fn cancelable_returns_cancelled() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let cancelled = AtomicBool::new(true);
        let cancel_token = || cancelled.load(Ordering::Relaxed);
        let result = render_pixel_buffer_parallel_rayon_cancelable(
            pixel_rect,
            &StubAlgorithm,
            &StubColourMap,
            &cancel_token,
        );
        assert!(matches!(
            result,
            Err(RenderPixelBufferCancelableError::Cancelled(_))
        ));
    }

    #[test]
    fn cancelable_produces_correct_output_when_not_cancelled() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let result = render_pixel_buffer_parallel_rayon_cancelable(
            pixel_rect,
            &StubAlgorithm,
            &StubColourMap,
            &NeverCancel,
        );
        let pb = result.unwrap();

        let expected: Vec<u8> = vec![
            0, 0, 0, 255, 1, 1, 1, 255, 2, 2, 2, 255, 1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255,
        ];
        assert_eq!(pb.buffer(), &expected);
    }

    #[test]
    fn error_displays_cancelled() {
        let err: RenderPixelBufferCancelableError<StubAlgError> =
            RenderPixelBufferCancelableError::Cancelled(Cancelled);
        assert_eq!(format!("{}", err), "operation cancelled");
    }

    #[test]
    fn error_displays_algorithm_error() {
        let err: RenderPixelBufferCancelableError<StubAlgError> =
            RenderPixelBufferCancelableError::Algorithm(StubAlgError);
        assert_eq!(format!("{}", err), "algorithm error: StubAlgError");
    }

    #[test]
    fn error_displays_colour_map_error() {
        let err: RenderPixelBufferCancelableError<StubAlgError> =
            RenderPixelBufferCancelableError::ColourMap("bad map".into());
        assert_eq!(format!("{}", err), "colour map error: bad map");
    }

    #[test]
    fn matches_old_two_stage_pipeline() {
        use crate::core::actions::generate_fractal::generate_fractal_parallel_rayon::generate_fractal_parallel_rayon;
        use crate::core::actions::generate_pixel_buffer::generate_pixel_buffer::generate_pixel_buffer;
        use crate::core::data::complex::Complex;
        use crate::core::data::complex_rect::ComplexRect;
        use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
        use crate::core::fractals::mandelbrot::colour_mapping::maps::ice::MandelbrotIceColourMap;

        let max_iterations = 100;
        let pixel_rect =
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 79, y: 59 }).unwrap();
        let complex_rect = ComplexRect::new(
            Complex { real: -2.5, imag: -1.0 },
            Complex { real: 1.0, imag: 1.0 },
        )
        .unwrap();
        let algorithm =
            MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations).unwrap();
        let colour_map = MandelbrotIceColourMap::new(max_iterations);

        // Old two-stage pipeline
        let iterations = generate_fractal_parallel_rayon(pixel_rect, &algorithm).unwrap();
        let old_pb = generate_pixel_buffer(iterations, &colour_map, pixel_rect).unwrap();

        // New single-pass pipeline
        let new_pb =
            render_pixel_buffer_parallel_rayon(pixel_rect, &algorithm, &colour_map).unwrap();

        assert_eq!(old_pb.buffer(), new_pb.buffer());
    }

    #[test]
    fn non_cancelable_error_displays_algorithm() {
        let err: RenderPixelBufferError<StubAlgError> =
            RenderPixelBufferError::Algorithm(StubAlgError);
        assert_eq!(format!("{}", err), "algorithm error: StubAlgError");
    }
}
