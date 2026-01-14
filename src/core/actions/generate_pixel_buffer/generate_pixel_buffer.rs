use crate::core::actions::cancellation::{
    CancelToken, Cancelled, NeverCancel, CANCEL_CHECK_INTERVAL_PIXELS,
};
use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::data::colour::Colour;
use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferData, PixelBufferError};
use crate::core::data::pixel_rect::PixelRect;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum GeneratePixelBufferError<ColourMapError: Error> {
    ColourMap(ColourMapError),
    PixelBuffer(PixelBufferError),
}

/// Error type for cancelable pixel buffer generation.
///
/// Distinguishes between processing errors and cancellation, allowing callers
/// to handle each case appropriately.
#[derive(Debug)]
pub enum GeneratePixelBufferCancelableError<ColourMapError: Error> {
    /// The operation was cancelled before completion.
    Cancelled(Cancelled),
    /// A colour mapping error occurred.
    ColourMap(ColourMapError),
    /// A pixel buffer construction error occurred.
    PixelBuffer(PixelBufferError),
}

impl<ColourMapError: Error> fmt::Display for GeneratePixelBufferCancelableError<ColourMapError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled(c) => write!(f, "{}", c),
            Self::ColourMap(err) => write!(f, "colour map error: {}", err),
            Self::PixelBuffer(err) => write!(f, "pixel buffer error: {}", err),
        }
    }
}

impl<ColourMapError: Error + 'static> Error for GeneratePixelBufferCancelableError<ColourMapError> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled(c) => Some(c),
            Self::ColourMap(err) => Some(err),
            Self::PixelBuffer(err) => Some(err),
        }
    }
}

impl<ColourMapError: Error> fmt::Display for GeneratePixelBufferError<ColourMapError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColourMap(err) => write!(f, "colour map error: {}", err),
            Self::PixelBuffer(err) => write!(f, "pixel buffer error: {}", err),
        }
    }
}

impl<ColourMapError: Error + 'static> Error for GeneratePixelBufferError<ColourMapError> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ColourMap(err) => Some(err),
            Self::PixelBuffer(err) => Some(err),
        }
    }
}

impl<ColourMapError: Error> From<PixelBufferError> for GeneratePixelBufferError<ColourMapError> {
    fn from(err: PixelBufferError) -> Self {
        Self::PixelBuffer(err)
    }
}

/// Generates a pixel buffer by mapping input values to colours.
///
/// For cancel-aware generation, use [`generate_pixel_buffer_cancelable`].
pub fn generate_pixel_buffer<CMap: ColourMap>(
    input: Vec<CMap::T>,
    mapper: &CMap,
    pixel_rect: PixelRect,
) -> Result<PixelBuffer, GeneratePixelBufferError<CMap::Failure>> {
    // Delegate to the cancel-aware implementation with NeverCancel
    generate_pixel_buffer_cancelable_impl(input, mapper, pixel_rect, &NeverCancel).map_err(|e| {
        match e {
            GeneratePixelBufferCancelableError::ColourMap(err) => {
                GeneratePixelBufferError::ColourMap(err)
            }
            GeneratePixelBufferCancelableError::PixelBuffer(err) => {
                GeneratePixelBufferError::PixelBuffer(err)
            }
            GeneratePixelBufferCancelableError::Cancelled(_) => {
                // NeverCancel never cancels, so this branch is unreachable
                unreachable!("NeverCancel token should never signal cancellation")
            }
        }
    })
}

/// Generates a pixel buffer with cancellation support.
///
/// Like [`generate_pixel_buffer`], but accepts a cancellation token that can
/// abort the operation early. Checks for cancellation periodically during
/// colour mapping.
///
/// Returns [`GeneratePixelBufferCancelableError::Cancelled`] if cancellation
/// was requested, which should be handled as expected control flow (not an
/// error to display).
#[allow(dead_code)]
pub fn generate_pixel_buffer_cancelable<CMap, C>(
    input: Vec<CMap::T>,
    mapper: &CMap,
    pixel_rect: PixelRect,
    cancel: &C,
) -> Result<PixelBuffer, GeneratePixelBufferCancelableError<CMap::Failure>>
where
    CMap: ColourMap,
    C: CancelToken,
{
    generate_pixel_buffer_cancelable_impl(input, mapper, pixel_rect, cancel)
}

/// Internal cancel-aware pixel buffer generation implementation.
///
/// Streams RGB bytes into a preallocated buffer while periodically checking
/// for cancellation. Checks `cancel.is_cancelled()` every
/// [`CANCEL_CHECK_INTERVAL_PIXELS`] pixels.
///
/// Preallocates the buffer to `pixel_rect.size() * 3` bytes to avoid
/// intermediate allocations and reduce wasted work on cancellation.
#[allow(dead_code)]
pub(crate) fn generate_pixel_buffer_cancelable_impl<CMap, C>(
    input: Vec<CMap::T>,
    mapper: &CMap,
    pixel_rect: PixelRect,
    cancel: &C,
) -> Result<PixelBuffer, GeneratePixelBufferCancelableError<CMap::Failure>>
where
    CMap: ColourMap,
    C: CancelToken,
{
    let buffer_size = (pixel_rect.size() * 3) as usize;
    let mut buffer: PixelBufferData = Vec::with_capacity(buffer_size);

    for (i, value) in input.into_iter().enumerate() {
        // Check cancellation every N pixels
        if i % CANCEL_CHECK_INTERVAL_PIXELS == 0 && cancel.is_cancelled() {
            return Err(GeneratePixelBufferCancelableError::Cancelled(Cancelled));
        }

        let Colour { r, g, b } = mapper
            .map(value)
            .map_err(GeneratePixelBufferCancelableError::ColourMap)?;

        buffer.push(r);
        buffer.push(g);
        buffer.push(b);
    }

    PixelBuffer::from_data(pixel_rect, buffer)
        .map_err(GeneratePixelBufferCancelableError::PixelBuffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
    use crate::core::data::colour::Colour;
    use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferData, PixelBufferError};
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug, PartialEq)]
    struct StubColourMapError {}

    impl std::fmt::Display for StubColourMapError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "StubColourMapError")
        }
    }

    impl Error for StubColourMapError {}

    #[derive(Debug)]
    struct StubColourMapSuccess {}

    impl ColourMap for StubColourMapSuccess {
        type T = u8;
        type Failure = StubColourMapError;

        fn map(&self, value: Self::T) -> Result<Colour, Self::Failure> {
            Ok(Colour {
                r: value,
                g: value,
                b: value,
            })
        }
    }

    #[derive(Debug, PartialEq)]
    struct StubColourMapFailure {}

    impl ColourMap for StubColourMapFailure {
        type T = u8;
        type Failure = StubColourMapError;

        fn map(&self, _: Self::T) -> Result<Colour, Self::Failure> {
            Err(StubColourMapError {})
        }
    }

    #[test]
    fn test_generates_pixel_buffer_correctly() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let expected_buffer: PixelBufferData =
            vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6];
        let expected_results = PixelBuffer::from_data(pixel_rect, expected_buffer).unwrap();
        let results = generate_pixel_buffer(input, &mapper, pixel_rect).unwrap();

        assert_eq!(results.buffer(), expected_results.buffer());
        assert_eq!(results.pixel_rect(), expected_results.pixel_rect());
        assert_eq!(results.buffer_size(), expected_results.buffer_size());
    }

    #[test]
    fn test_propagates_colour_map_failure() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapFailure {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 2 }).unwrap();
        let results = generate_pixel_buffer(input, &mapper, pixel_rect);

        assert!(matches!(
            results,
            Err(GeneratePixelBufferError::ColourMap(StubColourMapError {}))
        ));
    }

    #[test]
    fn test_pixel_rect_input_size_mismatch_returns_err() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 1, y: 1 }).unwrap();
        let results = generate_pixel_buffer(input, &mapper, pixel_rect);

        assert!(matches!(
            results,
            Err(GeneratePixelBufferError::PixelBuffer(
                PixelBufferError::BoundsMismatch {
                    pixel_rect_size: 12,
                    buffer_size: 18
                }
            ))
        ));
    }

    // Tests for cancelable implementation

    #[test]
    fn test_cancelable_generates_pixel_buffer_correctly() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let expected_buffer: PixelBufferData =
            vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6];
        let expected_results = PixelBuffer::from_data(pixel_rect, expected_buffer).unwrap();

        let results =
            generate_pixel_buffer_cancelable_impl(input, &mapper, pixel_rect, &NeverCancel)
                .unwrap();

        assert_eq!(results.buffer(), expected_results.buffer());
        assert_eq!(results.pixel_rect(), expected_results.pixel_rect());
        assert_eq!(results.buffer_size(), expected_results.buffer_size());
    }

    #[test]
    fn test_cancelable_returns_cancelled_when_token_is_cancelled() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let cancelled = AtomicBool::new(true);
        let cancel_token = || cancelled.load(Ordering::Relaxed);

        let result =
            generate_pixel_buffer_cancelable_impl(input, &mapper, pixel_rect, &cancel_token);

        assert!(matches!(
            result,
            Err(GeneratePixelBufferCancelableError::Cancelled(_))
        ));
    }

    #[test]
    fn test_cancelable_propagates_colour_map_failure() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapFailure {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 2 }).unwrap();

        let result =
            generate_pixel_buffer_cancelable_impl(input, &mapper, pixel_rect, &NeverCancel);

        assert!(matches!(
            result,
            Err(GeneratePixelBufferCancelableError::ColourMap(_))
        ));
    }

    #[test]
    fn test_cancelable_error_displays_cancelled() {
        let err: GeneratePixelBufferCancelableError<StubColourMapError> =
            GeneratePixelBufferCancelableError::Cancelled(Cancelled);
        assert_eq!(format!("{}", err), "operation cancelled");
    }

    #[test]
    fn test_cancelable_error_displays_colour_map_error() {
        let err: GeneratePixelBufferCancelableError<StubColourMapError> =
            GeneratePixelBufferCancelableError::ColourMap(StubColourMapError {});
        assert_eq!(format!("{}", err), "colour map error: StubColourMapError");
    }

    #[test]
    fn test_public_cancelable_api_works() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();

        // Test the public API function
        let result = generate_pixel_buffer_cancelable(input, &mapper, pixel_rect, &NeverCancel);

        assert!(result.is_ok());
        let pixel_buffer = result.unwrap();
        assert_eq!(pixel_buffer.buffer().len(), 18); // 6 pixels * 3 bytes
    }

    #[test]
    fn test_cancelled_does_not_create_pixel_buffer() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 2, y: 1 }).unwrap();
        let cancelled = AtomicBool::new(true);
        let cancel_token = || cancelled.load(Ordering::Relaxed);

        let result = generate_pixel_buffer_cancelable(input, &mapper, pixel_rect, &cancel_token);

        // Result should be Err(Cancelled), not a PixelBuffer
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(GeneratePixelBufferCancelableError::Cancelled(_))
        ));
    }
}
