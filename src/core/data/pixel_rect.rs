use crate::core::data::point::Point;
use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PixelRectError {
    InvalidSize { width: i32, height: i32 },
}

impl fmt::Display for PixelRectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { width, height } => {
                write!(f, "pixel rect size must be positive: {}x{}", width, height)
            }
        }
    }
}

impl Error for PixelRectError {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PixelRect {
    top_left: Point,
    bottom_right: Point,
}

impl PixelRect {
    pub fn new(top_left: Point, bottom_right: Point) -> Result<Self, PixelRectError> {
        let dx = (bottom_right.x as i64) - (top_left.x as i64);
        let dy = (bottom_right.y as i64) - (top_left.y as i64);

        let width = (dx + if dx >= 0 { 1 } else { -1 }) as i32;
        let height = (dy + if dy >= 0 { 1 } else { -1 }) as i32;

        if width < 1 || height < 1 {
            return Err(PixelRectError::InvalidSize { width, height });
        }

        Ok(Self {
            top_left,
            bottom_right,
        })
    }

    #[must_use]
    pub fn top_left(&self) -> Point {
        self.top_left
    }

    #[must_use]
    pub fn bottom_right(&self) -> Point {
        self.bottom_right
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        (self.bottom_right.x - self.top_left.x + 1) as u32
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        (self.bottom_right.y - self.top_left.y + 1) as u32
    }

    #[must_use]
    pub fn contains_point(&self, point: Point) -> bool {
        self.top_left.x <= point.x
            && self.top_left.y <= point.y
            && self.bottom_right.x >= point.x
            && self.bottom_right.y >= point.y
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn size(&self) -> u64 {
        (self.width() * self.height()) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::pixel_rect::PixelRectError;

    #[test]
    fn test_pixel_rect_new_valid() {
        let top_left = Point { x: 0, y: 0 };
        let bottom_right = Point { x: 100, y: 100 };

        let rect = PixelRect::new(top_left, bottom_right);
        let value = rect.unwrap();

        assert!(rect.is_ok());
        assert!(value.top_left() == top_left);
        assert!(value.bottom_right() == bottom_right);
    }

    #[test]
    fn test_pixel_rect_dimensions() {
        let rect = PixelRect::new(Point { x: -10, y: -20 }, Point { x: 110, y: 80 }).unwrap();

        assert_eq!(rect.width(), 121);
        assert_eq!(rect.height(), 101);
        assert_eq!(rect.size(), 12221);
    }

    #[test]
    fn test_single_pixel_rect_is_valid() {
        let rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 });

        assert!(rect.is_ok());
    }

    #[test]
    fn test_pixel_rect_dimensions_for_single_pixel() {
        let rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 }).unwrap();

        assert_eq!(rect.width(), 1);
        assert_eq!(rect.height(), 1);
        assert_eq!(rect.size(), 1);
    }

    #[test]
    fn test_pixel_rect_dimensions_must_be_positive() {
        let rect_negative_width = PixelRect::new(Point { x: 0, y: 0 }, Point { x: -1, y: 0 });

        let rect_negative_height = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: -1 });

        let rect_negative_width_and_height =
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: -1, y: -1 });

        assert_eq!(
            rect_negative_width,
            Err(PixelRectError::InvalidSize {
                width: -2,
                height: 1
            })
        );
        assert_eq!(
            rect_negative_height,
            Err(PixelRectError::InvalidSize {
                width: 1,
                height: -2
            })
        );
        assert_eq!(
            rect_negative_width_and_height,
            Err(PixelRectError::InvalidSize {
                width: -2,
                height: -2
            })
        );
    }

    #[test]
    fn test_pixel_rect_contains_point() {
        let rect = PixelRect::new(Point { x: -50, y: -50 }, Point { x: 100, y: 100 }).unwrap();

        assert!(rect.contains_point(Point { x: 50, y: 50 }));
        assert!(rect.contains_point(Point { x: -50, y: -50 }));
        assert!(rect.contains_point(Point { x: 100, y: 100 }));
        assert!(!rect.contains_point(Point { x: 101, y: 50 }));
        assert!(!rect.contains_point(Point { x: -51, y: 50 }));
        assert!(!rect.contains_point(Point { x: 50, y: -51 }));
        assert!(!rect.contains_point(Point { x: 50, y: 101 }));
    }
}
