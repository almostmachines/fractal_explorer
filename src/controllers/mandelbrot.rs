use std::sync::Arc;
use std::time::Instant;

use crate::core::data::complex::Complex;
use crate::core::data::point::Point;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
use crate::core::actions::generate_fractal::generate_fractal_parallel::generate_fractal_parallel;
use crate::core::actions::generate_pixel_buffer::generate_pixel_buffer::generate_pixel_buffer;
use crate::core::fractals::mandelbrot::colour_maps::blue_white_gradient::MandelbrotBlueWhiteGradient;
use crate::storage::write_ppm::write_ppm;

pub fn mandelbrot_controller() -> Result<(), Box<dyn std::error::Error>> {
    let width: i32 = 800;
    let height: i32 = 600;
    let max_iterations: u32 = 256;
    let filepath = "output/mandelbrot.ppm";

    let pixel_rect = PixelRect::new(
        Point { x: 0, y: 0 },
        Point { x: width, y: height },
    )?;

    // Classic Mandelbrot view
    let complex_rect = ComplexRect::new(
        Complex { real: -2.5, imag: -1.0  },
        Complex { real: 1.0, imag: 1.0  },
    )?;

    let num_threads: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!("Rendering Mandelbrot set...");
    println!("Image size: {}x{}", width, height);
    println!("Max iterations: {}", max_iterations);
    println!("Threads: {}", num_threads);

    let mandelbrot_algorithm = MandelbrotAlgorithm::new(pixel_rect, complex_rect, max_iterations)?;
    let algorithm_arc = Arc::new(mandelbrot_algorithm);
    let start = Instant::now();
    let fractal = generate_fractal_parallel(pixel_rect, algorithm_arc, num_threads)?;
    let parallel_duration = start.elapsed();

    println!("Duration:   {:?}", parallel_duration);

    let colour_map = MandelbrotBlueWhiteGradient::new(max_iterations);
    let pixel_buf = generate_pixel_buffer(fractal, &colour_map, pixel_rect)?;

    write_ppm(pixel_buf, filepath)?;
    println!("Saved to {}", filepath);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_controller_returns_ok() {
        let result = mandelbrot_controller();

        assert!(result.is_ok());
    }
}
