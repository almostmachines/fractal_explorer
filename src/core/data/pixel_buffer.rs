use std::error::Error;
use std::fmt;
use crate::core::data::colour::Colour;
use crate::core::data::point::Point;
use crate::core::data::pixel_rect::PixelRect;

fn pixel_rect_to_buffer_size(pixel_rect: PixelRect) -> usize {
    (pixel_rect.width() * pixel_rect.height() * 3) as usize
}

#[derive(Debug, Clone, PartialEq )]
pub enum PixelBufferError {
    #[allow(dead_code)]
    PixelOutsideBounds { pixel: Point, pixel_rect: PixelRect },
    BoundsMismatch { pixel_rect_size: usize, buffer_size: usize },
}

impl fmt::Display for PixelBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoundsMismatch { pixel_rect_size, buffer_size } => {
                write!(f, "pixel rect size {} does not match buffer size {}", pixel_rect_size, buffer_size)
            },
            Self::PixelOutsideBounds { pixel, pixel_rect } => {
                write!(
                    f, "pixel at x:{}, y:{} outside of PixelRect bounds top:{}, left:{}, bottom:{}, right:{}",
                    pixel.x,
                    pixel.y,
                    pixel_rect.top_left().y,
                    pixel_rect.top_left().x,
                    pixel_rect.bottom_right().y,
                    pixel_rect.bottom_right().x
                )
            }
        }
    }
}

impl Error for PixelBufferError {}

pub type PixelBufferData = Vec<u8>;

#[derive(Debug)]
pub struct PixelBuffer {
    pixel_rect: PixelRect,
    buffer: PixelBufferData,
}

impl PixelBuffer {
    #[allow(dead_code)]
    #[must_use]
    pub fn new(pixel_rect: PixelRect) -> Self {
        let total_bytes = pixel_rect_to_buffer_size(pixel_rect);

        Self {
            pixel_rect,
            buffer: vec![0; total_bytes]
        }
    }

    pub fn from_data(pixel_rect: PixelRect, buffer: PixelBufferData) -> Result<Self, PixelBufferError>  {
        let buffer_size = pixel_rect_to_buffer_size(pixel_rect);

        if buffer_size != buffer.len() {
            return Err(PixelBufferError::BoundsMismatch { pixel_rect_size: buffer_size, buffer_size: buffer.len() })
        }

        Ok(Self {
            pixel_rect,
            buffer
        })
    }

    #[must_use]
    pub fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }

    #[must_use]
    pub fn buffer(&self) -> &PixelBufferData {
        &self.buffer
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    #[allow(dead_code)]
    pub fn set_buffer(&mut self, buffer: PixelBufferData) -> Result<(), PixelBufferError> {
        let buffer_size = pixel_rect_to_buffer_size(self.pixel_rect);

        if buffer_size != buffer.len() {
            return Err(PixelBufferError::BoundsMismatch { pixel_rect_size: buffer_size, buffer_size: buffer.len() })
        }

        self.buffer = buffer;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_pixel(&mut self, pixel: Point, colour: Colour) -> Result<(), PixelBufferError> {
        let relative_x = pixel.x - self.pixel_rect.top_left().x;
        let relative_y = pixel.y - self.pixel_rect.top_left().y;
        let index = ((relative_y * self.pixel_rect.width() + relative_x) * 3) as usize;

        if !self.pixel_rect.contains_point(pixel) {
            return Err(PixelBufferError::PixelOutsideBounds { pixel, pixel_rect: self.pixel_rect })
        }

        self.buffer[index] = colour.r;
        self.buffer[index + 1] = colour.g;
        self.buffer[index + 2] = colour.b;

        Ok(())
    }
}
