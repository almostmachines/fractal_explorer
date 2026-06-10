use crate::core::actions::cancellation::{CancelToken, Cancelled};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::fractals::{
    julia::{algorithm::JuliaAlgorithm, colour_mapping::map::JuliaColourMap},
    mandelbrot::{colour_mapping::map::MandelbrotColourMap, render_path::MandelbrotRenderPath},
};
use crate::core::util::pixel_to_complex_coords::PixelToComplexCoordsError;

pub enum FractalConfig {
    Mandelbrot {
        colour_map: Box<dyn MandelbrotColourMap>,
        algorithm: MandelbrotRenderPath,
    },
    Julia {
        colour_map: Box<dyn JuliaColourMap>,
        algorithm: JuliaAlgorithm,
    },
}

impl FractalConfig {
    pub fn algorithm(
        &self,
    ) -> &(dyn FractalAlgorithm<Success = u32, Failure = PixelToComplexCoordsError> + Sync) {
        match self {
            FractalConfig::Mandelbrot { algorithm, .. } => algorithm,
            FractalConfig::Julia { algorithm, .. } => algorithm,
        }
    }

    pub fn colour_map(&self) -> &(dyn ColourMap<u32> + Send + Sync) {
        match self {
            FractalConfig::Mandelbrot { colour_map, .. } => colour_map.as_ref(),
            FractalConfig::Julia { colour_map, .. } => colour_map.as_ref(),
        }
    }

    /// Resolves per-render preparation work (e.g. the perturbation reference
    /// orbit) before pixels are computed, honouring cancellation. Runs on
    /// the render worker thread.
    pub fn prepare<C: CancelToken + ?Sized>(&self, cancel: &C) -> Result<(), Cancelled> {
        match self {
            FractalConfig::Mandelbrot { algorithm, .. } => algorithm.prepare(cancel),
            FractalConfig::Julia { .. } => Ok(()),
        }
    }
}

impl PartialEq for FractalConfig {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                FractalConfig::Mandelbrot { colour_map: cmap1, algorithm: alg1 },
                FractalConfig::Mandelbrot { colour_map: cmap2, algorithm: alg2 },
            ) => cmap1.kind() == cmap2.kind() && alg1 == alg2,
            (
                FractalConfig::Julia { colour_map: cmap1, algorithm: alg1 },
                FractalConfig::Julia { colour_map: cmap2, algorithm: alg2 },
            ) => cmap1.kind() == cmap2.kind() && alg1 == alg2,
            _ => false,
        }
    }
}
