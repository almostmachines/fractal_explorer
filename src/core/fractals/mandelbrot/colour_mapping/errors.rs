use std::{error::Error, fmt};

#[derive(Debug)]
pub enum MandelbrotColourMapErrors {
    IterationsExceedMax {
        iterations: u32,
        max_iterations: u32,
    },
    LutInvariantBroken {
        iterations: u32,
        max_iterations: u32,
    },
}

impl fmt::Display for MandelbrotColourMapErrors {
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
            Self::LutInvariantBroken {
                iterations,
                max_iterations,
            } => write!(
                f,
                "internal LUT invariant broken: iterations {} must be in 0..={} for lookup",
                iterations, max_iterations
            ),
        }
    }
}

impl Error for MandelbrotColourMapErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterations_exceed_max_still_formats_and_matches_as_before() {
        let err = MandelbrotColourMapErrors::IterationsExceedMax {
            iterations: 101,
            max_iterations: 100,
        };

        assert!(matches!(
            err,
            MandelbrotColourMapErrors::IterationsExceedMax {
                iterations: 101,
                max_iterations: 100
            }
        ));
        assert_eq!(err.to_string(), "iterations 101 exceeds maximum 100");
    }

    #[test]
    fn lut_invariant_broken_formats_as_internal_failure() {
        let err = MandelbrotColourMapErrors::LutInvariantBroken {
            iterations: 20,
            max_iterations: 10,
        };
        let message = err.to_string();

        assert!(matches!(
            err,
            MandelbrotColourMapErrors::LutInvariantBroken {
                iterations: 20,
                max_iterations: 10
            }
        ));
        assert!(message.contains("internal LUT invariant broken"));
        assert!(message.contains("20"));
        assert!(message.contains("10"));
    }
}
