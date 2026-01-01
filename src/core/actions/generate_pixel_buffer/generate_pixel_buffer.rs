use std::error::Error;
use std::fmt;
use crate::core::data::colour::Colour;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferError, PixelBufferData};
use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;

#[derive(Debug, PartialEq)]
pub enum GeneratePixelBufferError<ColourMapError: Error> {
    ColourMap(ColourMapError),
    PixelBuffer(PixelBufferError),
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

pub fn generate_pixel_buffer<CMap: ColourMap>(
    input: Vec<CMap::T>,
    mapper: &CMap,
    pixel_rect: PixelRect,
) -> Result<PixelBuffer, GeneratePixelBufferError<CMap::Failure>> {
    let colours: Result<Vec<Colour>, CMap::Failure> = input
        .into_iter()
        .map(|value| mapper.map(value))
        .collect();

    let buffer: PixelBufferData = colours
        .map_err(GeneratePixelBufferError::ColourMap)?
        .into_iter()
        .flat_map(|Colour { r, g, b }| [r, g, b])
        .collect();

    Ok(PixelBuffer::from_data(pixel_rect, buffer)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::colour::Colour;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferData, PixelBufferError};
    use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
    use crate::core::data::point::Point;

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
            Ok(Colour { r: value, g: value, b: value })
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
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 2 }).unwrap();
        let expected_buffer: PixelBufferData = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6];
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

        assert!(matches!(results, Err(GeneratePixelBufferError::ColourMap(StubColourMapError {}))));
    }

    #[test]
    fn test_pixel_rect_input_size_mismatch_returns_err() {
        let input: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mapper = StubColourMapSuccess {};
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 1, y: 1 }).unwrap();
        let results = generate_pixel_buffer(input, &mapper, pixel_rect);

        assert!(matches!(results, Err(GeneratePixelBufferError::PixelBuffer(PixelBufferError::BoundsMismatch { pixel_rect_size: 3, buffer_size: 18 }))));
    }
}
