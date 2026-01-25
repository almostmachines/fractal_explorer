# fractal_explorer

A Rust Mandelbrot fractal renderer with:

- A CLI “test generator” that writes a binary PPM (`output/mandelbrot.ppm`)
- An interactive GUI (feature-gated) for real-time exploration

The project uses a ports-and-adapters (hexagonal) architecture: core domain logic lives in `src/core/`, while UI/IO are implemented as adapters in `src/presenters/` and `src/input/`.

## Quickstart

Prerequisites: a recent Rust toolchain (edition 2024).

```bash
cargo build
cargo run                  # writes output/mandelbrot.ppm
```

The CLI run is a fixed “demo” render (currently 800×600 at 256 max iterations).

For faster renders, use release mode:

```bash
cargo run --release
```

To view the output PPM, you can use any PPM-capable viewer, or convert it (example with ImageMagick):

```bash
magick output/mandelbrot.ppm output/mandelbrot.png
```

## GUI

The GUI binary is behind the `gui` feature.

```bash
cargo run --bin gui --features gui
```

Current GUI controls:

- Max iterations (slider)
- Colour map (dropdown)
- Reset view

## Project layout

- `src/core/`: pure domain logic (fractal algorithms, data types, actions, utils)
- `src/controllers/`: orchestration for CLI/interactive flows + ports (interfaces) for presenters
- `src/presenters/`: output adapters (e.g., `presenters/file/ppm.rs` for PPM files)
- `src/input/gui/`: GUI app and command wiring (compiled only with `--features gui`)

See [ARCHITECTURE.md](ARCHITECTURE.md) for full details.

## Development

```bash
cargo test
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT License - see [LICENSE.txt](LICENSE.txt) for details.
