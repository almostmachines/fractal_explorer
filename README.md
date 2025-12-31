# Fractal Explorer

A Mandelbrot fractal renderer written in pure Rust with zero external dependencies. Uses **Ports & Adapters (Hexagonal Architecture)** for clean separation of concerns.

## Features

- Custom `Complex` number type implementation
- Mandelbrot escape-time algorithm (`z = z² + c`, escape when |z| > 2)
- PPM binary format (P6) image output
- Trait-based abstractions for algorithms and colour mapping
- Comprehensive validation with custom error types

## Quick Start

```bash
# Build and run
cargo run

# This generates output/mandelbrot.ppm (800x600, 256 iterations)
```

The default render produces a classic Mandelbrot view covering the complex plane from (-2.5, -1.0) to (1.0, 1.0).

## Build Commands

```bash
cargo build                 # Debug build
cargo build --release       # Optimized release build
cargo test                  # Run all tests
cargo check                 # Quick type checking
cargo clippy                # Lint warnings
```

## Architecture

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

```
src/
├── main.rs                     # Entry point
├── lib.rs                      # Library exports
├── controllers/                # Orchestrates the rendering pipeline
│   └── mandelbrot.rs
├── core/
│   ├── actions/                # Use cases with trait-based ports
│   │   ├── generate_fractal/   # Fractal computation action
│   │   │   └── ports/          # FractalAlgorithm trait
│   │   └── generate_pixel_buffer/  # Colour mapping action
│   │       └── ports/          # ColourMap trait
│   ├── data/                   # Domain types
│   │   ├── complex.rs          # Complex number type
│   │   ├── complex_rect.rs     # Region in complex plane
│   │   ├── pixel_rect.rs       # Image dimensions
│   │   ├── pixel_buffer.rs     # Raw pixel data
│   │   ├── point.rs            # 2D integer point
│   │   └── colour.rs           # RGB colour type
│   ├── fractals/               # Algorithm implementations
│   │   └── mandelbrot/
│   │       ├── algorithm.rs    # Mandelbrot computation
│   │       └── colour_maps/    # Colour mapping strategies
│   └── util/                   # Coordinate conversion utilities
└── storage/                    # PPM image file output
    └── write_ppm.rs
```

## Extending

The hexagonal architecture makes it easy to add:

- **New fractal algorithms**: Implement the `FractalAlgorithm` trait
- **New colour schemes**: Implement the `ColourMap` trait
- **New output formats**: Add adapters in `storage/`
