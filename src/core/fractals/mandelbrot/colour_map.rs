use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::data::colour::Colour;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MandelbrotColourMapKind {
    BlueWhiteGradient,
    FireGradient,
}

pub trait MandelbrotColourMap: ColourMap<u32> + Send + Sync {
    fn kind(&self) -> MandelbrotColourMapKind;
}

impl ColourMap<u32> for Box<dyn MandelbrotColourMap> {
    fn map(&self, value: u32) -> Result<Colour, Box<dyn Error>> {
        (**self).map(value)
    }

    fn display_name(&self) -> &str {
        (**self).display_name()
    }
}
