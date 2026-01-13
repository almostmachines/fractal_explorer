use crate::core::data::complex::Complex;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PixelToComplexCoordsError {
    PointOutsideRect { point: Point, pixel_rect: PixelRect },
}

impl fmt::Display for PixelToComplexCoordsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointOutsideRect { point, pixel_rect } => {
                write!(
                    f,
                    "point (x: {}, y: {}) is outside the rectangle with coords top-left: (x: {}, y: {}) bottom-right: (x: {}, y: {})",
                    point.x,
                    point.y,
                    pixel_rect.top_left().x,
                    pixel_rect.top_left().y,
                    pixel_rect.bottom_right().x,
                    pixel_rect.bottom_right().y
                )
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
        return Err(PixelToComplexCoordsError::PointOutsideRect {
            point: pixel_position,
            pixel_rect,
        });
    }

    let relative_pixel_x = (pixel_position.x - pixel_rect.top_left().x) as f64;
    let relative_pixel_y = (pixel_position.y - pixel_rect.top_left().y) as f64;
    let real = complex_rect.top_left().real
        + (relative_pixel_x / (pixel_rect.width() - 1) as f64) * complex_rect.width();
    let imag = complex_rect.top_left().imag
        + (relative_pixel_y / (pixel_rect.height() - 1) as f64) * complex_rect.height();

    Ok(Complex { real, imag })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_to_complex_top_left() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.0,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let result = pixel_to_complex_coords(Point { x: 0, y: 0 }, pixel_rect, complex_rect);

        assert_eq!(result.unwrap().real, -2.0);
        assert_eq!(result.unwrap().imag, -1.0);
    }

    #[test]
    fn test_pixel_to_complex_bottom_right() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.0,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let result = pixel_to_complex_coords(Point { x: 100, y: 100 }, pixel_rect, complex_rect);
        assert_eq!(result.unwrap().real, 1.0);
        assert_eq!(result.unwrap().imag, 1.0);
    }

    #[test]
    fn test_pixel_to_complex_center() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.0,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let result = pixel_to_complex_coords(Point { x: 50, y: 50 }, pixel_rect, complex_rect);

        assert_eq!(result.unwrap().real, 0.0);
        assert_eq!(result.unwrap().imag, 0.0);
    }

    #[test]
    fn test_pixel_outside_complex_fails() {
        let point1 = Point { x: 150, y: 150 };
        let point2 = Point { x: -10, y: -10 };

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 100, y: 100 }).unwrap();

        let complex_rect = ComplexRect::new(
            Complex {
                real: -1.0,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        let result1 = pixel_to_complex_coords(point1, pixel_rect, complex_rect);
        let result2 = pixel_to_complex_coords(point2, pixel_rect, complex_rect);

        assert_eq!(
            result1,
            Err(PixelToComplexCoordsError::PointOutsideRect {
                point: point1,
                pixel_rect
            })
        );
        assert_eq!(
            result2,
            Err(PixelToComplexCoordsError::PointOutsideRect {
                point: point2,
                pixel_rect
            })
        );
    }
}
