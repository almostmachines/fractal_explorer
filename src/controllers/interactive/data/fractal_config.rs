use crate::core::fractals::{julia::{algorithm::JuliaAlgorithm, colour_mapping::map::JuliaColourMap}, mandelbrot::{algorithm::MandelbrotAlgorithm, colour_mapping::map::MandelbrotColourMap}};

pub enum FractalConfig {
    Mandelbrot {
        colour_map: Box<dyn MandelbrotColourMap>,
        algorithm: MandelbrotAlgorithm,
    },
    Julia {
        colour_map: Box<dyn JuliaColourMap>,
        algorithm: JuliaAlgorithm,
    },
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
