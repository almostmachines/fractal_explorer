use crate::core::data::colour::Colour;
use std::error::Error;

pub trait ColourMap {
    type T;
    type Failure: Error;

    fn map(&self, value: Self::T) -> Result<Colour, Self::Failure>;
}
