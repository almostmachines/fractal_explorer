use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum MandelbrotError {
    ZeroMaxIterationsError,
}

impl fmt::Display for MandelbrotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaxIterationsError => {
                write!(f, "Maximum iterations must be greater than zero")
            }
        }
    }
}

impl Error for MandelbrotError {}
