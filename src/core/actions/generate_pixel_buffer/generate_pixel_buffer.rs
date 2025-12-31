use std::error::Error;
use std::fmt;
use crate::core::data::colour::Colour;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferError, PixelBufferData};
use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;

#[derive(Debug)]
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
