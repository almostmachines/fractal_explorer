use crate::controllers::interactive::ColourSchemeKind;
use crate::core::data::fractal::Fractal;
use crate::core::data::fractal_params::FractalParams;
use crate::core::data::pixel_rect::PixelRect;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    pub pixel_rect: PixelRect,
    pub fractal: Fractal,
    pub params: FractalParams,
    pub colour_scheme: ColourSchemeKind,
}
