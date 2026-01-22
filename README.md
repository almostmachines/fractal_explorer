# Fractal Explorer

A Rust-based Mandelbrot fractal renderer with both CLI and interactive GUI capabilities. Features parallel rendering, multiple colour maps, and real-time exploration.

## Features

- **Mandelbrot Set Rendering** - Classic fractal generation with configurable iterations
- **Multiple Colour Maps** - Fire gradient (red to white) and blue-white gradient
- **Parallel Processing** - Work-stealing parallelism via Rayon for fast rendering
- **Interactive GUI** - Change fractal parameters and colour maps in real-time
- **PPM Output** - Simple, portable image format for CLI renders

## Quick Start

```bash
# Generate a fractal image (CLI)
cargo run --release

# Launch the interactive GUI
cargo run --release --features gui --bin gui
```

The CLI outputs to `output/mandelbrot.ppm`. You can view PPM files with most image viewers or convert them using ImageMagick:

```bash
convert output/mandelbrot.ppm output/mandelbrot.png
```

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Optimized build
cargo test               # Run all tests
cargo run                # Generate fractal (CLI)
cargo run --features gui --bin gui  # Interactive GUI
```

## GUI Controls

The interactive GUI provides:

- **Iterations Slider** - Adjust detail level (1-1000)
- **Colour Map Selection** - Switch between colour schemes
- **Reset View** - Return to default view
- **Render Timing** - See how long each frame takes

## Project Structure

```
src/
├── main.rs                 # CLI entry point
├── bin/gui.rs              # GUI entry point
├── core/
│   ├── data/               # Complex numbers, pixel buffers, colours
│   ├── fractals/mandelbrot/
│   │   ├── algorithm.rs    # Mandelbrot iteration
│   │   └── colour_mapping/ # Colour map implementations
│   └── actions/            # Fractal generation (serial & parallel)
├── controllers/            # Application orchestration
├── input/gui/              # GUI input handling (winit + egui)
└── storage/                # PPM file output
```

## Technical Details

### Rendering

Four parallel strategies are implemented:
- **Rayon** (default) - Work-stealing thread pool
- **Scoped Threads** - Manual thread management
- **Arc/Channels** - Atomic communication
- **Serial** - Single-threaded baseline

### GUI Architecture

The GUI uses a multi-threaded design:
- Main thread handles UI rendering via egui/winit
- Background worker computes fractals
- Request coalescing prevents redundant work during rapid input
- Generation IDs ensure stale frames are discarded

## Dependencies

- **rayon** - Parallel iteration
- **winit** - Window management (GUI)
- **pixels** - Framebuffer rendering (GUI)
- **egui** - Immediate-mode UI (GUI)
- **wgpu** - Graphics backend (GUI)

## License

MIT
