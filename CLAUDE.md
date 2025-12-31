# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                 # Debug build
cargo build --release       # Optimized release build
cargo test                  # Run all tests
cargo test <test_name>      # Run a single test
cargo check                 # Quick type checking
cargo clippy                # Lint warnings
cargo run                   # Generate mandelbrot.ppm in output/
```

## Architecture Overview

A Mandelbrot fractal renderer using **Ports & Adapters (Hexagonal Architecture)** with zero external dependencies.

### Data Flow

```
PixelRect + ComplexRect
    → FractalAlgorithm trait (Mandelbrot implementation)
    → generate_fractal() → Vec<u32> (iterations per pixel)
    → ColourMap trait (blue_white_gradient implementation)
    → generate_pixel_buffer() → PixelBuffer
    → write_ppm() → output/mandelbrot.ppm
```

### Module Structure

- **controllers/**: Entry points that orchestrate the rendering pipeline
- **core/actions/**: Use cases with trait-based ports for algorithm and colour map abstraction
  - `generate_fractal/ports/FractalAlgorithm` - trait for fractal computation
  - `generate_pixel_buffer/ports/ColourMap` - trait for iteration-to-colour mapping
- **core/data/**: Domain types (Complex, Point, PixelRect, ComplexRect, PixelBuffer, Colour)
- **core/fractals/**: Algorithm implementations (Mandelbrot, colour maps)
- **core/util/**: Coordinate conversion between pixel and complex space
- **storage/**: PPM image file output

### Key Implementation Details

- Custom `Complex` number type (not using external crate)
- Mandelbrot escape-time algorithm: `z = z² + c`, escape when |z| > 2
- PPM binary format (P6) for portable image output
- Comprehensive validation with custom error types per domain
