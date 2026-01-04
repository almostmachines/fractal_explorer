# Fractal Explorer

A Mandelbrot set renderer written in Rust that generates PPM images. Features one serial and multiple parallel computation strategies and a clean ports & adapters architecture for easy extension.

## Features

- Mandelbrot set fractal generation
- Multiple parallelization strategies (Rayon, scoped threads, Arc-based)
- Blue-white gradient colour mapping
- PPM image output
- Trait-based architecture for adding new fractals and colour schemes

## Requirements

- Rust 2024 edition
- Rayon 1.10

## Building and Running

```bash
# Debug build
cargo build

# Optimized build
cargo build --release

# Generate fractal image
cargo run

# Run with release optimizations
cargo run --release
```

The program outputs a Mandelbrot set image to `output/mandelbrot.ppm`.

## Output

Default configuration:
- Image size: 800x600 pixels
- Complex plane: real [-2.5, 1.0], imaginary [-1.0, 1.0]
- Max iterations: 256
- Colour scheme: Blue-white gradient

## Architecture

The project uses a ports & adapters pattern with trait-based abstractions.

### Data Flow

```
mandelbrot_controller
    │
    ├─► generate_fractal_parallel_rayon() → Vec<u32> iteration counts
    │     Uses FractalAlgorithm trait
    │
    ├─► generate_pixel_buffer() → PixelBuffer RGB data
    │     Uses ColourMap trait
    │
    └─► write_ppm() → output/mandelbrot.ppm
```

### Module Structure

```
src/
├── main.rs                 # Entry point
├── lib.rs                  # Library exports
├── controllers/            # Orchestration logic
├── core/
│   ├── data/               # Data types (Complex, Point, Colour, etc.)
│   ├── fractals/           # Fractal algorithms and colour maps
│   ├── actions/            # Fractal generation and pixel buffer creation
│   └── util/               # Coordinate conversion utilities
└── storage/                # PPM file output
```

### Key Traits

- **FractalAlgorithm**: Implement to add new fractal types
- **ColourMap**: Implement to add new colour schemes

## Extending

### Adding a New Fractal

1. Implement the `FractalAlgorithm` trait in `src/core/fractals/`
2. Create an algorithm struct with the required computation logic
3. Use it with any of the generation functions

### Adding a New Colour Map

1. Implement the `ColourMap` trait in a new module
2. Define the `map_iteration_count_to_colour()` method
3. Use it with `generate_pixel_buffer()`

## Testing

```bash
# Run all tests (87 total)
cargo test

# Run unit tests only
cargo test --lib

# Run a specific test
cargo test <test_name>
```

## License

MIT
