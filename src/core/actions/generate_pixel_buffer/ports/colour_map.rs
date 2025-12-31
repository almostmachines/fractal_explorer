use std::error::Error;
use crate::core::data::colour::Colour;

pub trait ColourMap {
    type T;
    type Failure: Error;

    fn map(&self, value: Self::T) -> Result<Colour, Self::Failure>;
}
