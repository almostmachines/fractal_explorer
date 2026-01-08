# Project Architecture

```
.
├── AGENTS.md                          # AI agent guidelines
├── ARCHITECTURE.md                    # This file
├── Cargo.lock                         # Dependency lock file
├── Cargo.toml                         # Project manifest
├── CLAUDE.md                          # Claude Code instructions
├── LICENSE.txt                        # MIT license
├── output/                            # Generated image output
├── README.md                          # Project documentation
└── src/                               # Source code root
    ├── controllers/                   # Orchestration logic
    │   ├── mandelbrot.rs              # Main fractal pipeline
    │   └── mod.rs                     # Module exports
    ├── core/                          # Core domain logic
    │   ├── actions/                   # Business operations
    │   │   ├── generate_fractal/      # Fractal computation
    │   │   │   ├── generate_fractal_parallel_arc.rs      # Arc-based threading
    │   │   │   ├── generate_fractal_parallel_rayon.rs    # Rayon work-stealing (default)
    │   │   │   ├── generate_fractal_parallel_scoped_threads.rs  # Scoped thread impl
    │   │   │   ├── generate_fractal_serial.rs            # Single-threaded impl
    │   │   │   ├── mod.rs                                # Module exports
    │   │   │   └── ports/                                # Trait definitions
    │   │   │       ├── fractal_algorithm.rs              # FractalAlgorithm trait
    │   │   │       └── mod.rs                            # Module exports
    │   │   ├── generate_pixel_buffer/                    # RGB conversion
    │   │   │   ├── generate_pixel_buffer.rs              # Iteration to colour
    │   │   │   ├── mod.rs                                # Module exports
    │   │   │   └── ports/                                # Trait definitions
    │   │   │       ├── colour_map.rs                     # ColourMap trait
    │   │   │       └── mod.rs                            # Module exports
    │   │   └── mod.rs                                    # Module exports
    │   ├── data/                      # Data structures
    │   │   ├── colour.rs              # RGB colour type
    │   │   ├── complex_rect.rs        # Complex plane bounds
    │   │   ├── complex.rs             # Complex number type
    │   │   ├── mod.rs                 # Module exports
    │   │   ├── pixel_buffer.rs        # Image buffer type
    │   │   ├── pixel_rect.rs          # Pixel coordinate bounds
    │   │   └── point.rs               # 2D point type
    │   ├── fractals/                  # Fractal implementations
    │   │   ├── mandelbrot/            # Mandelbrot set
    │   │   │   ├── algorithm.rs       # Escape time algorithm
    │   │   │   ├── colour_maps/       # Colour schemes
    │   │   │   │   ├── blue_white_gradient.rs  # Default gradient
    │   │   │   │   └── mod.rs                  # Module exports
    │   │   │   └── mod.rs             # Module exports
    │   │   └── mod.rs                 # Module exports
    │   ├── mod.rs                     # Module exports
    │   └── util/                      # Utility functions
    │       ├── calculate_bands_in_pixel_rect.rs          # Band subdivision
    │       ├── calculate_threads_for_pixel_rect_banding.rs  # Thread count logic
    │       ├── mod.rs                                    # Module exports
    │       └── pixel_to_complex_coords.rs                # Coordinate conversion
    ├── lib.rs                         # Library entry point
    ├── main.rs                        # Binary entry point
    └── storage/                       # Output handling
        ├── mod.rs                     # Module exports
        └── write_ppm.rs               # PPM file writer
```
