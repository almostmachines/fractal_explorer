use std::error::Error;
use std::fmt;
use crate::core::data::point::Point;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::complex::Complex;
use crate::core::data::complex_rect::ComplexRect;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PixelToComplexCoordsError {
    PointOutsideRect { point: Point, rect: PixelRect },
}

impl fmt::Display for PixelToComplexCoordsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointOutsideRect { point, rect } => {
                write!(f, "point (x: {}, y: {}) is outside the rectangle with coords top-left: (x: {}, y: {}) bottom-right: (x: {}, y: {})", point.x, point.y, rect.top_left().x, rect.top_left().y, rect.bottom_right().x, rect.bottom_right().y)
            }
        }
    }
}

impl Error for PixelToComplexCoordsError {}

pub fn pixel_to_complex_coords(
    pixel_position: Point,
    pixel_rect: PixelRect,
    complex_rect: ComplexRect,
) -> Result<Complex, PixelToComplexCoordsError> {
    if !pixel_rect.contains_point(pixel_position) {
        return Err(PixelToComplexCoordsError::PointOutsideRect { point: pixel_position, rect: pixel_rect })
    }

    let relative_pixel_x = (pixel_position.x - pixel_rect.top_left().x) as f64;
    let relative_pixel_y = (pixel_position.y - pixel_rect.top_left().y) as f64;
    let real = complex_rect.top_left().real + (relative_pixel_x / pixel_rect.width() as f64) * complex_rect.width();
    let imag = complex_rect.top_left().imag + (relative_pixel_y / pixel_rect.height() as f64) * complex_rect.height();

    Ok(Complex { real, imag })
}
