use crate::core::actions::cancellation::{CancelToken, Cancelled};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
use crate::core::fractals::mandelbrot::perturbation::algorithm::MandelbrotPerturbationAlgorithm;
use crate::core::util::pixel_to_complex_coords::PixelToComplexCoordsError;

/// How a Mandelbrot frame gets computed: directly in f64 (fast and exact at
/// shallow zoom) or via a perturbation reference orbit (deep zoom).
#[derive(Debug, PartialEq)]
pub enum MandelbrotRenderPath {
    Direct(MandelbrotAlgorithm),
    Perturbation(MandelbrotPerturbationAlgorithm),
}

impl MandelbrotRenderPath {
    /// Resolves any per-render preparation (the perturbation reference
    /// orbit), honouring cancellation.
    pub fn prepare<C: CancelToken + ?Sized>(&self, cancel: &C) -> Result<(), Cancelled> {
        match self {
            Self::Direct(_) => Ok(()),
            Self::Perturbation(algorithm) => algorithm.prepare(cancel),
        }
    }

    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Direct(_) => "CPU f64",
            Self::Perturbation(_) => "CPU perturbation",
        }
    }
}

impl FractalAlgorithm for MandelbrotRenderPath {
    type Success = u32;
    type Failure = PixelToComplexCoordsError;

    fn compute(&self, pixel: Point) -> Result<Self::Success, Self::Failure> {
        match self {
            Self::Direct(algorithm) => algorithm.compute(pixel),
            Self::Perturbation(algorithm) => algorithm.compute(pixel),
        }
    }

    fn compute_row_segment_into(
        &self,
        y: i32,
        x_start: i32,
        x_end: i32,
        output: &mut Vec<Self::Success>,
    ) -> Result<(), Self::Failure> {
        match self {
            Self::Direct(algorithm) => algorithm.compute_row_segment_into(y, x_start, x_end, output),
            Self::Perturbation(algorithm) => {
                algorithm.compute_row_segment_into(y, x_start, x_end, output)
            }
        }
    }

    fn pixel_rect(&self) -> PixelRect {
        match self {
            Self::Direct(algorithm) => algorithm.pixel_rect(),
            Self::Perturbation(algorithm) => algorithm.pixel_rect(),
        }
    }
}
