use crate::core::actions::generate_pixel_buffer::ports::colour_map::{ColourMap, ColourMapError};
use crate::core::data::colour::Colour;
use crate::core::fractals::julia::colour_mapping::kinds::JuliaColourMapKinds;
use crate::core::fractals::julia::colour_mapping::map::JuliaColourMap;
use crate::core::fractals::julia::colour_mapping::errors::JuliaColourMapErrors;
use crate::core::util::iteration_colour_lut::IterationColourLut;

#[derive(Debug)]
pub struct JuliaIceColourMap {
    max_iterations: u32,
    lut: IterationColourLut,
}

impl ColourMap<u32> for JuliaIceColourMap {
    fn map(&self, iterations: u32) -> Result<Colour, ColourMapError> {
        if iterations > self.max_iterations {
            return Err(Box::new(JuliaColourMapErrors::IterationsExceedMax {
                iterations,
                max_iterations: self.max_iterations,
            }));
        }

        if let Some(colour) = self.lut.get(iterations) {
            return Ok(colour);
        }

        debug_assert!(
            false,
            "LUT invariant broken: iterations <= max_iterations but LUT had no entry"
        );
        Err(Box::new(JuliaColourMapErrors::LutInvariantBroken {
            iterations,
            max_iterations: self.max_iterations,
        }))
    }

    fn display_name(&self) -> &str {
        self.kind().display_name()
    }
}

impl JuliaColourMap for JuliaIceColourMap {
    fn kind(&self) -> JuliaColourMapKinds {
        JuliaColourMapKinds::BlueWhiteGradient
    }
}

impl JuliaIceColourMap {
    #[must_use]
    pub fn new(max_iterations: u32) -> Self {
        let lut = IterationColourLut::new(max_iterations, Self::colour_from_t);
        Self {
            max_iterations,
            lut,
        }
    }

    fn colour_from_t(t: f64) -> Colour {
        let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u8;
        let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
        let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;

        Colour { r, g, b }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_colour(max_iterations: u32, iterations: u32) -> Colour {
        if iterations == max_iterations {
            return Colour { r: 0, g: 0, b: 0 };
        }

        let t = iterations as f64 / max_iterations as f64;
        let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u8;
        let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
        let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;

        Colour { r, g, b }
    }

    fn assert_colour_eq(actual: Colour, expected: Colour) {
        assert_eq!(actual.r, expected.r);
        assert_eq!(actual.g, expected.g);
        assert_eq!(actual.b, expected.b);
    }

    #[test]
    fn test_map_returns_black_at_max_iterations() {
        let mapper = JuliaIceColourMap::new(100);
        let colour = mapper.map(100).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_returns_black_at_zero_iterations() {
        let mapper = JuliaIceColourMap::new(100);
        let colour = mapper.map(0).unwrap();

        assert_eq!(colour.r, 0);
        assert_eq!(colour.g, 0);
        assert_eq!(colour.b, 0);
    }

    #[test]
    fn test_map_midpoint_gradient() {
        let mapper = JuliaIceColourMap::new(100);
        let colour = mapper.map(50).unwrap();

        assert_eq!(colour.r, 143);
        assert_eq!(colour.g, 239);
        assert_eq!(colour.b, 135);
    }

    #[test]
    fn test_map_returns_error_when_iterations_exceed_max() {
        let mapper = JuliaIceColourMap::new(100);
        let result = mapper.map(101);
        let err = result.expect_err("expected error when iterations exceed max");

        assert!(matches!(
            err.downcast_ref::<JuliaColourMapErrors>(),
            Some(JuliaColourMapErrors::IterationsExceedMax {
                iterations: 101,
                max_iterations: 100
            })
        ));
    }

    #[test]
    fn lut_size_matches_max_plus_one() {
        let mapper = JuliaIceColourMap::new(100);

        assert_eq!(mapper.lut.len(), 101);
    }

    #[test]
    fn map_with_max_zero_is_black_for_zero_and_errors_for_positive() {
        let mapper = JuliaIceColourMap::new(0);

        let black = mapper.map(0).expect("zero iteration should be valid");
        assert_colour_eq(black, Colour { r: 0, g: 0, b: 0 });

        let err = mapper
            .map(1)
            .expect_err("positive iteration must exceed max when max=0");

        assert!(matches!(
            err.downcast_ref::<JuliaColourMapErrors>(),
            Some(JuliaColourMapErrors::IterationsExceedMax {
                iterations: 1,
                max_iterations: 0
            })
        ));
    }

    #[test]
    fn lut_matches_reference_formula_for_sample_points() {
        let max_iterations = 100;
        let mapper = JuliaIceColourMap::new(max_iterations);

        for iterations in [0, 1, 25, 50, 75, 99, 100] {
            let expected = reference_colour(max_iterations, iterations);
            let actual = mapper.map(iterations).expect("sample point should map");
            assert_colour_eq(actual, expected);
        }
    }

    #[test]
    fn lut_matches_reference_formula_for_entire_domain_small_max() {
        let max_iterations = 32;
        let mapper = JuliaIceColourMap::new(max_iterations);

        for iterations in 0..=max_iterations {
            let expected = reference_colour(max_iterations, iterations);
            let actual = mapper
                .map(iterations)
                .expect("iteration in domain should map");
            assert_colour_eq(actual, expected);
        }
    }
}
