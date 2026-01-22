use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::data::colour::Colour;
use crate::core::fractals::mandelbrot::colour_map::{MandelbrotColourMap, MandelbrotColourMapKind};
use crate::core::fractals::mandelbrot::colour_maps::errors::MandelbrotColourMapErrors;
use std::error::Error;

#[derive(Debug)]
pub struct MandelbrotFireGradient {
    max_iterations: u32,
}

impl ColourMap<u32> for MandelbrotFireGradient {
    fn map(&self, iterations: u32) -> Result<Colour, Box<dyn Error>> {
        if iterations > self.max_iterations {
            return Err(Box::new(MandelbrotColourMapErrors::IterationsExceedMax {
                iterations,
                max_iterations: self.max_iterations,
            }));
        }

        if iterations == self.max_iterations {
            return Ok(Colour { r: 0, g: 0, b: 0 });
        }

        let t = iterations as f64 / self.max_iterations as f64;

        let (r, g, b) = if t < 0.25 {
            let local_t = t / 0.25;
            (
                (local_t * 255.0) as u8,
                0,
                0,
            )
        } else if t < 0.5 {
            let local_t = (t - 0.25) / 0.25;
            (
                255,
                (local_t * 165.0) as u8,
                0,
            )
        } else if t < 0.75 {
            let local_t = (t - 0.5) / 0.25;
            (
                255,
                (165.0 + local_t * 90.0) as u8,
                0,
            )
        } else {
            let local_t = (t - 0.75) / 0.25;
            (
                255,
                255,
                (local_t * 255.0) as u8,
            )
        };

        Ok(Colour { r, g, b })
    }

    fn display_name(&self) -> &str {
        self.kind().display_name()
    }
}

impl MandelbrotColourMap for MandelbrotFireGradient {
    fn kind(&self) -> MandelbrotColourMapKind {
        MandelbrotColourMapKind::FireGradient
    }
}

impl MandelbrotFireGradient {
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
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(100).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_returns_black_at_zero_iterations() {
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(0).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_quarter_is_red() {
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(25).unwrap();

        assert_eq!(colour.r, 255);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_half_is_orange() {
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(50).unwrap();

        assert_eq!(colour.r, 255);
        assert_eq!(colour.g, 165);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_three_quarters_is_yellow() {
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(75).unwrap();

        assert_eq!(colour.r, 255);
        assert_eq!(colour.g, 255);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_near_max_is_near_white() {
        let mapper = MandelbrotFireGradient::new(100);
        let colour = mapper.map(99).unwrap();

        assert_eq!(colour.r, 255);
        assert_eq!(colour.g, 255);
        assert!(colour.b > 240); // Nearly white
    }
}
