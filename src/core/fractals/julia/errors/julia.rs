use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum JuliaError {
    ZeroMaxIterationsError,
}

impl fmt::Display for JuliaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaxIterationsError => {
                write!(f, "Maximum iterations must be greater than zero")
            }
        }
    }
}

impl Error for JuliaError {}
