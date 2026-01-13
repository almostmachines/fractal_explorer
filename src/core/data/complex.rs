use std::ops::{Add, Mul};

// implement Complex instead of using the num-complex trait for learning
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    #[must_use]
    pub fn magnitude_squared(&self) -> f64 {
        self.real * self.real + self.imag * self.imag
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magnitude_squared() {
        let c = Complex {
            real: 3.0,
            imag: 4.0,
        };
        assert_eq!(c.magnitude_squared(), 25.0); // 3² + 4² = 25
    }

    #[test]
    fn test_magnitude_squared_negative_real() {
        let c = Complex {
            real: -3.0,
            imag: 4.0,
        };
        assert_eq!(c.magnitude_squared(), 25.0); // 3² + 4² = 25
    }

    #[test]
    fn test_magnitude_squared_negative_imag() {
        let c = Complex {
            real: 3.0,
            imag: -4.0,
        };
        assert_eq!(c.magnitude_squared(), 25.0); // 3² + 4² = 25
    }

    #[test]
    fn test_magnitude_squared_negative_real_and_imag() {
        let c = Complex {
            real: -3.0,
            imag: -4.0,
        };
        assert_eq!(c.magnitude_squared(), 25.0); // 3² + 4² = 25
    }

    #[test]
    fn test_magnitude_squared_zero() {
        let c = Complex {
            real: 0.0,
            imag: 0.0,
        };
        assert_eq!(c.magnitude_squared(), 0.0);
    }

    #[test]
    fn test_add() {
        let a = Complex {
            real: 1.0,
            imag: 2.0,
        };
        let b = Complex {
            real: 3.0,
            imag: 4.0,
        };
        let result = a + b;
        assert_eq!(result.real, 4.0);
        assert_eq!(result.imag, 6.0);
    }

    #[test]
    fn test_add_negative() {
        let a = Complex {
            real: 1.0,
            imag: 2.0,
        };
        let b = Complex {
            real: -3.0,
            imag: -7.0,
        };
        let result = a + b;
        assert_eq!(result.real, -2.0);
        assert_eq!(result.imag, -5.0);
    }

    #[test]
    fn test_mul() {
        // (1 + 2i) * (3 + 4i) = 3 + 4i + 6i + 8i² = 3 + 10i - 8 = -5 + 10i
        let a = Complex {
            real: 1.0,
            imag: 2.0,
        };
        let b = Complex {
            real: 3.0,
            imag: 4.0,
        };
        let result = a * b;
        assert_eq!(result.real, -5.0);
        assert_eq!(result.imag, 10.0);
    }

    #[test]
    fn test_mul_negative() {
        // (1 + 2i) * (3 + 4i) = 3 + 4i + 6i + 8i² = 3 + 10i - 8 = -5 + 10i
        let a = Complex {
            real: 1.0,
            imag: 2.0,
        };
        let b = Complex {
            real: -3.0,
            imag: -4.0,
        };
        let result = a * b;
        assert_eq!(result.real, 5.0);
        assert_eq!(result.imag, -10.0);
    }

    #[test]
    fn test_mul_by_zero() {
        let a = Complex {
            real: 5.0,
            imag: 3.0,
        };
        let zero = Complex {
            real: 0.0,
            imag: 0.0,
        };
        let result = a * zero;
        assert_eq!(result.real, 0.0);
        assert_eq!(result.imag, 0.0);
    }

    #[test]
    fn test_square() {
        // (2 + 3i)² = 4 + 12i + 9i² = 4 + 12i - 9 = -5 + 12i
        let c = Complex {
            real: 2.0,
            imag: 3.0,
        };
        let result = c * c;
        assert_eq!(result.real, -5.0);
        assert_eq!(result.imag, 12.0);
    }
}
