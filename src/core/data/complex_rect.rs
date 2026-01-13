use crate::core::data::complex::Complex;
use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ComplexRectError {
    InvalidSize { width: f64, height: f64 },
}

impl fmt::Display for ComplexRectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { width, height } => {
                write!(
                    f,
                    "complex rect size must be positive: {}x{}",
                    width, height
                )
            }
        }
    }
}

impl Error for ComplexRectError {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ComplexRect {
    top_left: Complex,
    bottom_right: Complex,
}

impl ComplexRect {
    pub fn new(top_left: Complex, bottom_right: Complex) -> Result<Self, ComplexRectError> {
        let width = bottom_right.real - top_left.real;
        let height = bottom_right.imag - top_left.imag;

        if width <= 0.0 || height <= 0.0 {
            return Err(ComplexRectError::InvalidSize { width, height });
        }

        Ok(Self {
            top_left,
            bottom_right,
        })
    }

    #[must_use]
    pub fn top_left(&self) -> Complex {
        self.top_left
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn bottom_right(&self) -> Complex {
        self.bottom_right
    }

    #[must_use]
    pub fn width(&self) -> f64 {
        self.bottom_right.real - self.top_left.real
    }

    #[must_use]
    pub fn height(&self) -> f64 {
        self.bottom_right.imag - self.top_left.imag
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn contains_point(&self, point: Complex) -> bool {
        self.top_left.real <= point.real
            && self.top_left.imag <= point.imag
            && self.bottom_right.real >= point.real
            && self.bottom_right.imag >= point.imag
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn size(&self) -> u64 {
        (self.width() * self.height()).abs() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_rect_new_valid() {
        let top_left = Complex {
            real: -2.0,
            imag: -1.0,
        };
        let bottom_right = Complex {
            real: 1.0,
            imag: 1.0,
        };

        let rect = ComplexRect::new(top_left, bottom_right);
        let value = rect.unwrap();

        assert!(rect.is_ok());
        assert!(value.top_left() == top_left);
        assert!(value.bottom_right() == bottom_right);
    }

    #[test]
    fn test_complex_rect_dimensions_must_be_positive() {
        let rect_zero_width = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: 0.0,
                imag: 100.0,
            },
        );

        let rect_negative_width = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: -100.0,
                imag: 10.0,
            },
        );

        let rect_zero_height = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: 100.0,
                imag: 0.0,
            },
        );

        let rect_negative_height = ComplexRect::new(
            Complex {
                real: 0.0,
                imag: 0.0,
            },
            Complex {
                real: 100.0,
                imag: -10.0,
            },
        );

        let rect_zero_width_and_height = ComplexRect::new(
            Complex {
                real: 2.0,
                imag: 2.0,
            },
            Complex {
                real: 2.0,
                imag: 2.0,
            },
        );

        let rect_negative_width_and_height = ComplexRect::new(
            Complex {
                real: 2.0,
                imag: 2.0,
            },
            Complex {
                real: -2.0,
                imag: -2.0,
            },
        );

        assert_eq!(
            rect_zero_width,
            Err(ComplexRectError::InvalidSize {
                width: 0.0,
                height: 100.0
            })
        );
        assert_eq!(
            rect_negative_width,
            Err(ComplexRectError::InvalidSize {
                width: -100.0,
                height: 10.0
            })
        );
        assert_eq!(
            rect_zero_height,
            Err(ComplexRectError::InvalidSize {
                width: 100.0,
                height: 0.0
            })
        );
        assert_eq!(
            rect_negative_height,
            Err(ComplexRectError::InvalidSize {
                width: 100.0,
                height: -10.0
            })
        );
        assert_eq!(
            rect_zero_width_and_height,
            Err(ComplexRectError::InvalidSize {
                width: 0.0,
                height: 0.0
            })
        );
        assert_eq!(
            rect_negative_width_and_height,
            Err(ComplexRectError::InvalidSize {
                width: -4.0,
                height: -4.0
            })
        );
    }

    #[test]
    fn test_complex_rect_dimensions() {
        let rect = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap();

        assert_eq!(rect.width(), 3.5);
        assert_eq!(rect.height(), 2.0);
    }

    #[test]
    fn test_complex_rect_contains_point() {
        let rect = ComplexRect::new(
            Complex {
                real: -10.0,
                imag: -5.0,
            },
            Complex {
                real: 100.0,
                imag: 200.0,
            },
        )
        .unwrap();

        assert!(rect.contains_point(Complex {
            real: 50.0,
            imag: 50.0
        }));
        assert!(rect.contains_point(Complex {
            real: -10.0,
            imag: 0.0
        }));
        assert!(rect.contains_point(Complex {
            real: 100.0,
            imag: 200.0
        }));
        assert!(!rect.contains_point(Complex {
            real: 101.0,
            imag: 50.0
        }));
        assert!(!rect.contains_point(Complex {
            real: -11.0,
            imag: 50.0
        }));
        assert!(!rect.contains_point(Complex {
            real: 50.0,
            imag: -6.0
        }));
        assert!(!rect.contains_point(Complex {
            real: 50.0,
            imag: 201.0
        }));
    }
}
