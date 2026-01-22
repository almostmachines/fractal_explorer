use crate::core::data::colour::Colour;
use std::error::Error;

pub trait ColourMap<T>: Send + Sync {
    fn map(&self, value: T) -> Result<Colour, Box<dyn Error>>;
    fn display_name(&self) -> &str;
}
