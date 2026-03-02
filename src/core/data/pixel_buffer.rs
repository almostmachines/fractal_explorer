use crate::core::data::colour::Colour;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use std::error::Error;
use std::fmt;

fn pixel_rect_to_buffer_size(pixel_rect: PixelRect) -> usize {
    (pixel_rect.width() * pixel_rect.height()) as usize * PixelBuffer::BYTES_PER_PIXEL
}

#[derive(Debug, Clone, PartialEq)]
pub enum PixelBufferError {
    PixelOutsideBounds { pixel: Point, pixel_rect: PixelRect },
    BoundsMismatch {
        pixel_rect_size: usize,
        buffer_size: usize,
    },
}

impl fmt::Display for PixelBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoundsMismatch {
                pixel_rect_size,
                buffer_size,
            } => {
                write!(
                    f,
                    "pixel rect size {} does not match buffer size {}",
                    pixel_rect_size, buffer_size
                )
            }
            Self::PixelOutsideBounds { pixel, pixel_rect } => {
                write!(
                    f,
                    "pixel at x:{}, y:{} outside of PixelRect bounds top:{}, left:{}, bottom:{}, right:{}",
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

/// Opaque RGBA pixel buffer in row-major order (`r, g, b, a`) with `a = 255`.
#[derive(Debug)]
pub struct PixelBuffer {
    pixel_rect: PixelRect,
    buffer: PixelBufferData,
}

impl PixelBuffer {
    pub const BYTES_PER_PIXEL: usize = 4;
    pub const ALPHA_OPAQUE: u8 = 255;

    fn normalize_alpha(buffer: &mut PixelBufferData) {
        for pixel in buffer.chunks_exact_mut(Self::BYTES_PER_PIXEL) {
            pixel[Self::BYTES_PER_PIXEL - 1] = Self::ALPHA_OPAQUE;
        }
    }

    #[must_use]
    pub fn new(pixel_rect: PixelRect) -> Self {
        let total_bytes = pixel_rect_to_buffer_size(pixel_rect);
        let mut buffer = vec![0; total_bytes];
        Self::normalize_alpha(&mut buffer);

        Self { pixel_rect, buffer }
    }

    pub fn from_data(
        pixel_rect: PixelRect,
        mut buffer: PixelBufferData,
    ) -> Result<Self, PixelBufferError> {
        let buffer_size = pixel_rect_to_buffer_size(pixel_rect);

        if buffer_size != buffer.len() {
            return Err(PixelBufferError::BoundsMismatch {
                pixel_rect_size: buffer_size,
                buffer_size: buffer.len(),
            });
        }

        Self::normalize_alpha(&mut buffer);

        Ok(Self { pixel_rect, buffer })
    }

    #[must_use]
    pub fn pixel_rect(&self) -> PixelRect {
        self.pixel_rect
    }

    #[must_use]
    pub fn buffer(&self) -> &PixelBufferData {
        &self.buffer
    }

    #[must_use]
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    pub fn set_buffer(&mut self, mut buffer: PixelBufferData) -> Result<(), PixelBufferError> {
        let buffer_size = pixel_rect_to_buffer_size(self.pixel_rect);

        if buffer_size != buffer.len() {
            return Err(PixelBufferError::BoundsMismatch {
                pixel_rect_size: buffer_size,
                buffer_size: buffer.len(),
            });
        }

        Self::normalize_alpha(&mut buffer);
        self.buffer = buffer;
        Ok(())
    }

    pub fn set_pixel(&mut self, pixel: Point, colour: Colour) -> Result<(), PixelBufferError> {
        if !self.pixel_rect.contains_point(pixel) {
            return Err(PixelBufferError::PixelOutsideBounds {
                pixel,
                pixel_rect: self.pixel_rect,
            });
        }

        let relative_x = (pixel.x - self.pixel_rect.top_left().x) as u32;
        let relative_y = (pixel.y - self.pixel_rect.top_left().y) as u32;
        let index =
            ((relative_y * self.pixel_rect.width() + relative_x) as usize) * Self::BYTES_PER_PIXEL;

        self.buffer[index] = colour.r;
        self.buffer[index + 1] = colour.g;
        self.buffer[index + 2] = colour.b;
        self.buffer[index + 3] = Self::ALPHA_OPAQUE;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_pixel_rect(width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )
        .unwrap()
    }

    fn create_offset_pixel_rect(x: i32, y: i32, width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x, y },
            Point {
                x: x + width - 1,
                y: y + height - 1,
            },
        )
        .unwrap()
    }

    fn expected_size(width: usize, height: usize) -> usize {
        width * height * PixelBuffer::BYTES_PER_PIXEL
    }

    fn assert_alpha_is_opaque(buffer: &[u8]) {
        for pixel in buffer.chunks_exact(PixelBuffer::BYTES_PER_PIXEL) {
            assert_eq!(pixel[3], PixelBuffer::ALPHA_OPAQUE);
        }
    }

    #[test]
    fn test_new_creates_black_opaque_buffer() {
        let pixel_rect = create_pixel_rect(10, 10);
        let buffer = PixelBuffer::new(pixel_rect);

        assert_eq!(buffer.pixel_rect(), pixel_rect);
        assert_eq!(buffer.buffer_size(), expected_size(10, 10));
        for pixel in buffer.buffer().chunks_exact(PixelBuffer::BYTES_PER_PIXEL) {
            assert_eq!(pixel[0], 0);
            assert_eq!(pixel[1], 0);
            assert_eq!(pixel[2], 0);
            assert_eq!(pixel[3], PixelBuffer::ALPHA_OPAQUE);
        }
    }

    #[test]
    fn test_new_calculates_correct_buffer_size() {
        let pixel_rect = create_pixel_rect(100, 50);
        let buffer = PixelBuffer::new(pixel_rect);

        assert_eq!(buffer.buffer_size(), expected_size(100, 50));
    }

    #[test]
    fn test_from_data_valid() {
        let pixel_rect = create_pixel_rect(2, 2);
        let data: Vec<u8> = vec![
            255, 0, 0, 10, // pixel (0,0) - red
            0, 255, 0, 20, // pixel (1,0) - green
            0, 0, 255, 30, // pixel (0,1) - blue
            255, 255, 0, 40, // pixel (1,1) - yellow
        ];

        let buffer = PixelBuffer::from_data(pixel_rect, data);

        assert!(buffer.is_ok());
        let buffer = buffer.unwrap();
        assert_eq!(buffer.pixel_rect(), pixel_rect);
        assert_eq!(
            buffer.buffer(),
            &vec![
                255,
                0,
                0,
                PixelBuffer::ALPHA_OPAQUE,
                0,
                255,
                0,
                PixelBuffer::ALPHA_OPAQUE,
                0,
                0,
                255,
                PixelBuffer::ALPHA_OPAQUE,
                255,
                255,
                0,
                PixelBuffer::ALPHA_OPAQUE,
            ]
        );
    }

    #[test]
    fn test_from_data_buffer_too_small() {
        let pixel_rect = create_pixel_rect(2, 2);
        let data: Vec<u8> = vec![255, 0, 0]; // Only 3 bytes, need 16

        let result = PixelBuffer::from_data(pixel_rect, data);

        assert_eq!(
            result.unwrap_err(),
            PixelBufferError::BoundsMismatch {
                pixel_rect_size: expected_size(2, 2),
                buffer_size: 3
            }
        );
    }

    #[test]
    fn test_from_data_buffer_too_large() {
        let pixel_rect = create_pixel_rect(2, 2);
        let data: Vec<u8> = vec![0; expected_size(2, 2) * 2];

        let result = PixelBuffer::from_data(pixel_rect, data);

        assert_eq!(
            result.unwrap_err(),
            PixelBufferError::BoundsMismatch {
                pixel_rect_size: expected_size(2, 2),
                buffer_size: expected_size(2, 2) * 2
            }
        );
    }

    #[test]
    fn test_from_data_empty_buffer_for_valid_rect() {
        let pixel_rect = create_pixel_rect(2, 2);
        let data: Vec<u8> = vec![];

        let result = PixelBuffer::from_data(pixel_rect, data);

        assert_eq!(
            result.unwrap_err(),
            PixelBufferError::BoundsMismatch {
                pixel_rect_size: expected_size(2, 2),
                buffer_size: 0
            }
        );
    }

    #[test]
    fn test_pixel_rect_getter() {
        let pixel_rect = create_offset_pixel_rect(10, 20, 30, 40);
        let buffer = PixelBuffer::new(pixel_rect);

        assert_eq!(buffer.pixel_rect(), pixel_rect);
    }

    #[test]
    fn test_buffer_getter() {
        let pixel_rect = create_pixel_rect(2, 2);
        let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 44, 55, 66, 77];
        let buffer = PixelBuffer::from_data(pixel_rect, data).unwrap();

        assert_eq!(
            buffer.buffer(),
            &vec![
                1,
                2,
                3,
                PixelBuffer::ALPHA_OPAQUE,
                5,
                6,
                7,
                PixelBuffer::ALPHA_OPAQUE,
                9,
                10,
                11,
                PixelBuffer::ALPHA_OPAQUE,
                44,
                55,
                66,
                PixelBuffer::ALPHA_OPAQUE,
            ]
        );
    }

    #[test]
    fn test_buffer_size_getter() {
        let pixel_rect = create_pixel_rect(5, 7);
        let buffer = PixelBuffer::new(pixel_rect);

        assert_eq!(buffer.buffer_size(), expected_size(5, 7));
    }

    #[test]
    fn test_set_buffer_valid() {
        let pixel_rect = create_pixel_rect(2, 2);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let new_data: Vec<u8> = vec![
            255, 1, 2, 1, 3, 255, 4, 2, 5, 6, 255, 3, 255, 255, 255, 4,
        ];

        let result = buffer.set_buffer(new_data);

        assert!(result.is_ok());
        assert_alpha_is_opaque(buffer.buffer());
        assert_eq!(
            buffer.buffer(),
            &vec![
                255,
                1,
                2,
                PixelBuffer::ALPHA_OPAQUE,
                3,
                255,
                4,
                PixelBuffer::ALPHA_OPAQUE,
                5,
                6,
                255,
                PixelBuffer::ALPHA_OPAQUE,
                255,
                255,
                255,
                PixelBuffer::ALPHA_OPAQUE,
            ]
        );
    }

    #[test]
    fn test_set_buffer_wrong_size() {
        let pixel_rect = create_pixel_rect(2, 2);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let new_data: Vec<u8> = vec![255; 6]; // Wrong size

        let result = buffer.set_buffer(new_data);

        assert_eq!(
            result,
            Err(PixelBufferError::BoundsMismatch {
                pixel_rect_size: expected_size(2, 2),
                buffer_size: 6
            })
        );
    }

    #[test]
    fn test_set_buffer_preserves_original_on_error() {
        let pixel_rect = create_pixel_rect(2, 2);
        let original_data: Vec<u8> = vec![
            1,
            2,
            3,
            PixelBuffer::ALPHA_OPAQUE,
            4,
            5,
            6,
            PixelBuffer::ALPHA_OPAQUE,
            7,
            8,
            9,
            PixelBuffer::ALPHA_OPAQUE,
            10,
            11,
            12,
            PixelBuffer::ALPHA_OPAQUE,
        ];
        let mut buffer = PixelBuffer::from_data(pixel_rect, original_data.clone()).unwrap();
        let wrong_data: Vec<u8> = vec![255; 6];
        let _ = buffer.set_buffer(wrong_data);

        assert_eq!(buffer.buffer(), &original_data);
    }

    #[test]
    fn test_set_pixel_valid() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let red = Colour { r: 255, g: 0, b: 0 };
        let result = buffer.set_pixel(Point { x: 1, y: 1 }, red);

        assert!(result.is_ok());
        assert_eq!(buffer.buffer()[16], 255);
        assert_eq!(buffer.buffer()[17], 0);
        assert_eq!(buffer.buffer()[18], 0);
        assert_eq!(buffer.buffer()[19], PixelBuffer::ALPHA_OPAQUE);
    }

    #[test]
    fn test_set_pixel_top_left_corner() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let green = Colour { r: 0, g: 255, b: 0 };
        let result = buffer.set_pixel(Point { x: 0, y: 0 }, green);

        assert!(result.is_ok());
        assert_eq!(buffer.buffer()[0], 0);
        assert_eq!(buffer.buffer()[1], 255);
        assert_eq!(buffer.buffer()[2], 0);
        assert_eq!(buffer.buffer()[3], PixelBuffer::ALPHA_OPAQUE);
    }

    #[test]
    fn test_set_pixel_bottom_right_corner() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let blue = Colour { r: 0, g: 0, b: 255 };
        let result = buffer.set_pixel(Point { x: 2, y: 2 }, blue);

        assert!(result.is_ok());
        assert_eq!(buffer.buffer()[32], 0);
        assert_eq!(buffer.buffer()[33], 0);
        assert_eq!(buffer.buffer()[34], 255);
        assert_eq!(buffer.buffer()[35], PixelBuffer::ALPHA_OPAQUE);
    }

    #[test]
    fn test_set_pixel_with_offset_rect() {
        let pixel_rect = create_offset_pixel_rect(10, 20, 3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);

        let white = Colour {
            r: 255,
            g: 255,
            b: 255,
        };

        let result = buffer.set_pixel(Point { x: 11, y: 21 }, white);

        assert!(result.is_ok());
        assert_eq!(buffer.buffer()[16], 255);
        assert_eq!(buffer.buffer()[17], 255);
        assert_eq!(buffer.buffer()[18], 255);
        assert_eq!(buffer.buffer()[19], PixelBuffer::ALPHA_OPAQUE);
    }

    #[test]
    fn test_set_pixel_outside_bounds_right() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let colour = Colour { r: 255, g: 0, b: 0 };
        let result = buffer.set_pixel(Point { x: 5, y: 1 }, colour);

        assert_eq!(
            result,
            Err(PixelBufferError::PixelOutsideBounds {
                pixel: Point { x: 5, y: 1 },
                pixel_rect
            })
        );
    }

    #[test]
    fn test_set_pixel_outside_bounds_bottom() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let colour = Colour { r: 255, g: 0, b: 0 };
        let result = buffer.set_pixel(Point { x: 1, y: 5 }, colour);

        assert_eq!(
            result,
            Err(PixelBufferError::PixelOutsideBounds {
                pixel: Point { x: 1, y: 5 },
                pixel_rect
            })
        );
    }

    #[test]
    fn test_set_pixel_outside_bounds_negative() {
        let pixel_rect = create_pixel_rect(3, 3);
        let mut buffer = PixelBuffer::new(pixel_rect);
        let colour = Colour { r: 255, g: 0, b: 0 };
        let result = buffer.set_pixel(Point { x: -1, y: -1 }, colour);

        assert_eq!(
            result,
            Err(PixelBufferError::PixelOutsideBounds {
                pixel: Point { x: -1, y: -1 },
                pixel_rect
            })
        );
    }

    #[test]
    fn test_set_multiple_pixels() {
        let pixel_rect = create_pixel_rect(2, 2);
        let mut buffer = PixelBuffer::new(pixel_rect);

        buffer
            .set_pixel(Point { x: 0, y: 0 }, Colour { r: 255, g: 0, b: 0 })
            .unwrap();

        buffer
            .set_pixel(Point { x: 1, y: 0 }, Colour { r: 0, g: 255, b: 0 })
            .unwrap();

        buffer
            .set_pixel(Point { x: 0, y: 1 }, Colour { r: 0, g: 0, b: 255 })
            .unwrap();

        buffer
            .set_pixel(
                Point { x: 1, y: 1 },
                Colour {
                    r: 255,
                    g: 255,
                    b: 0,
                },
            )
            .unwrap();

        let expected: Vec<u8> = vec![
            255,
            0,
            0,
            PixelBuffer::ALPHA_OPAQUE, // (0,0) red
            0,
            255,
            0,
            PixelBuffer::ALPHA_OPAQUE, // (1,0) green
            0,
            0,
            255,
            PixelBuffer::ALPHA_OPAQUE, // (0,1) blue
            255,
            255,
            0,
            PixelBuffer::ALPHA_OPAQUE, // (1,1) yellow
        ];

        assert_eq!(buffer.buffer(), &expected);
        assert_alpha_is_opaque(buffer.buffer());
    }
}
