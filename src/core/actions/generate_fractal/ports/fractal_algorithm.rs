use std::error::Error;
use crate::core::data::point::Point;

pub trait FractalAlgorithm {
    type Success;
    type Failure: Error;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure>;
}
