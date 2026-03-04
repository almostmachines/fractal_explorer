use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use std::error::Error;

pub trait FractalAlgorithm {
    type Success;
    type Failure: Error;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure>;
    fn pixel_rect(&self) -> PixelRect;

    fn compute_row_segment_into(
        &self,
        y: i32,
        x_start: i32,
        x_end: i32,
        output: &mut Vec<Self::Success>,
    ) -> Result<(), Self::Failure> {
        for x in x_start..=x_end {
            output.push(self.compute(Point { x, y })?);
        }

        Ok(())
    }
}
