use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::data::colour::Colour;
use crate::core::fractals::mandelbrot::colour_map::{MandelbrotColourMap, MandelbrotColourMapKind};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct MandelbrotBlueWhiteGradient {
    max_iterations: u32,
}

#[derive(Debug)]
pub enum MandelbrotGradientError {
    IterationsExceedMax {
        iterations: u32,
        max_iterations: u32,
    },
}

impl fmt::Display for MandelbrotGradientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IterationsExceedMax {
                iterations,
                max_iterations,
            } => {
                write!(
                    f,
                    "iterations {} exceeds maximum {}",
                    iterations, max_iterations
                )
            }
        }
    }
}

impl Error for MandelbrotGradientError {}

impl ColourMap<u32> for MandelbrotBlueWhiteGradient {
    fn map(&self, iterations: u32) -> Result<Colour, Box<dyn Error>> {
        if iterations > self.max_iterations {
            return Err(Box::new(MandelbrotGradientError::IterationsExceedMax {
                iterations,
                max_iterations: self.max_iterations,
            }));
        }

        if iterations == self.max_iterations {
            Ok(Colour { r: 0, g: 0, b: 0 })
        } else {
            // Simple approach: use iteration count to create a gradient
            let t = iterations as f64 / self.max_iterations as f64;

            // A nice blue-to-white gradient
            let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u8;
            let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
            let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;

            Ok(Colour { r, g, b })
        }
    }

    fn display_name(&self) -> &str {
        "Blue-white gradient"
    }
}

impl MandelbrotColourMap for MandelbrotBlueWhiteGradient {
    fn kind(&self) -> MandelbrotColourMapKind {
        MandelbrotColourMapKind::BlueWhiteGradient
    }
}

impl MandelbrotBlueWhiteGradient {
    #[must_use]
    pub fn new(max_iterations: u32) -> Self {
        Self { max_iterations }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_returns_black_at_max_iterations() {
        let mapper = MandelbrotBlueWhiteGradient::new(100);
        let colour = mapper.map(100).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_returns_black_at_zero_iterations() {
        let mapper = MandelbrotBlueWhiteGradient::new(100);
        let colour = mapper.map(0).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_midpoint_gradient() {
        let mapper = MandelbrotBlueWhiteGradient::new(100);
        let colour = mapper.map(50).unwrap();

        assert_eq!(colour.r, 143);
        assert_eq!(colour.g, 239);
        assert_eq!(colour.b, 135);
    }

    #[test]
    fn test_map_quarter_gradient() {
        let mapper = MandelbrotBlueWhiteGradient::new(100);
        let colour = mapper.map(25).unwrap();

        assert_eq!(colour.r, 26);
        assert_eq!(colour.g, 134);
        assert_eq!(colour.b, 228);
    }

    // #[test]
    // fn test_map_returns_error_when_iterations_exceed_max() {
    //     let mapper = MandelbrotBlueWhiteGradient::new(100);
    //     let result = mapper.map(101);
    //
    //     assert!(matches!(
    //         result,
    //         Err(MandelbrotGradientError::IterationsExceedMax {
    //             iterations: 101,
    //             max_iterations: 100
    //         })
    //     ));
    // }
}
