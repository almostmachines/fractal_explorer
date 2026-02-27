use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use std::error::Error;

pub trait FractalAlgorithm {
    type Success;
    type Failure: Error;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure>;
    fn pixel_rect(&self) -> PixelRect;
}
