use std::{error::Error, fmt};

#[derive(Debug)]
pub enum MandelbrotColourMapErrors {
    IterationsExceedMax {
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
        }
    }
}

impl Error for MandelbrotColourMapErrors {}
