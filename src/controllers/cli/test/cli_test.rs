use std::{path::Path, time::Instant};

use crate::{controllers::ports::file_presenter::FilePresenterPort, core::{actions::{generate_fractal::generate_fractal_parallel_rayon::generate_fractal_parallel_rayon, generate_pixel_buffer::generate_pixel_buffer::generate_pixel_buffer}, data::{complex::Complex, complex_rect::ComplexRect, pixel_buffer::PixelBuffer, pixel_rect::PixelRect, point::Point}, fractals::mandelbrot::{algorithm::MandelbrotAlgorithm, colour_mapping::maps::fire_gradient::MandelbrotFireGradient}}};

pub struct CliTestController<P: FilePresenterPort> {
    presenter: P,
    buffer: Option<PixelBuffer>,
}

impl<P: FilePresenterPort> CliTestController<P> {
    pub fn new(presenter: P) -> Self {
        Self {
            presenter,
            buffer: None,
        }
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let width: i32 = 800;
        let height: i32 = 600;
        let max_iterations: u32 = 256;

        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )?;

        let complex_rect = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )?;

        println!("Rendering Mandelbrot set...");
        println!("Image size: {}x{}", width, height);
        println!("Max iterations: {}", max_iterations);

        let algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations)?;
        let start = Instant::now();
        let fractal = generate_fractal_parallel_rayon(pixel_rect, &algorithm)?;
        let duration = start.elapsed();

        println!("Duration:   {:?}", duration);

        let colour_map = MandelbrotFireGradient::new(max_iterations);

        self.buffer = Some(generate_pixel_buffer(fractal, &colour_map, pixel_rect)?);

        Ok(())
    }

    pub fn write(&self, filepath: impl AsRef<Path>) -> std::io::Result<()> {
        if let Some(buffer) = &self.buffer {
            self.presenter.present(buffer, filepath)?
        }

        Ok(())
    }
}
