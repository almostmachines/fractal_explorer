use crate::core::fractals::mandelbrot::{algorithm::MandelbrotAlgorithm, colour_map::MandelbrotColourMap};

pub enum FractalConfig {
    Mandelbrot {
        colour_map: Box<dyn MandelbrotColourMap>,
        algorithm: MandelbrotAlgorithm,
    }
}

impl PartialEq for FractalConfig {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                FractalConfig::Mandelbrot { colour_map: cmap1, algorithm: alg1 },
                FractalConfig::Mandelbrot { colour_map: cmap2, algorithm: alg2 },
            ) => cmap1.kind() == cmap2.kind() && alg1 == alg2,
        }
    }
}
