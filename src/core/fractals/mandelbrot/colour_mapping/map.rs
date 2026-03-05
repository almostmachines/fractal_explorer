use crate::core::actions::generate_pixel_buffer::ports::colour_map::{ColourMap, ColourMapError};
use crate::core::data::colour::Colour;
use crate::core::fractals::mandelbrot::colour_mapping::kinds::MandelbrotColourMapKinds;

pub trait MandelbrotColourMap: ColourMap<u32> + Send + Sync {
    fn kind(&self) -> MandelbrotColourMapKinds;
}

impl ColourMap<u32> for Box<dyn MandelbrotColourMap> {
    fn map(&self, value: u32) -> Result<Colour, ColourMapError> {
        (**self).map(value)
    }

    fn display_name(&self) -> &str {
        (**self).display_name()
    }
}
