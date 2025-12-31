# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` is the binary entry point; `src/lib.rs` exposes reusable components.
- `src/controllers/` orchestrates the rendering pipeline (CLI entry flow).
- `src/core/` holds domain logic:
  - `actions/` use cases with ports (`generate_fractal`, `generate_pixel_buffer`).
  - `data/` domain types (Complex, Point, PixelRect, PixelBuffer, Colour, etc.).
  - `fractals/` algorithm implementations and colour maps.
  - `util/` coordinate conversion helpers.
- `src/storage/` contains file output (`write_ppm.rs`).
- `output/` is where rendered images are written (e.g., `output/mandelbrot.ppm`).

## Build, Test, and Development Commands
- `cargo build` — debug build.
- `cargo build --release` — optimized build.
- `cargo run` — render a Mandelbrot image to `output/mandelbrot.ppm`.
- `cargo test` — run all unit tests.
- `cargo test <test_name>` — run a single test by name.
- `cargo check` — fast type-check.
- `cargo clippy` — lint warnings and basic code quality checks.

## Coding Style & Naming Conventions
- Use Rust 2024 edition conventions and standard 4-space indentation.
- `snake_case` for functions/modules; `UpperCamelCase` for types/traits; `SCREAMING_SNAKE_CASE` for constants.
- Keep domain logic in `src/core/` and I/O in `controllers/` or `storage/`.
- Prefer small, composable functions and explicit error types in domain code.
- This repository currently has zero external dependencies; avoid adding new crates unless there is a clear need.

## Testing Guidelines
- Tests are inline `mod tests` blocks in the modules they cover (see `src/core/data/*.rs`).
- Use `#[test]` with descriptive names, e.g., `complex_adds_imag_part`.
- Run the full suite with `cargo test` before submitting changes.

## Commit & Pull Request Guidelines
- Commit messages are short, imperative summaries (e.g., “Pass PixelBuffer to write_ppm by value”).
- Keep commits focused on a single change or refactor.
- PRs should include: a brief summary, key commands run (e.g., `cargo test`), and any generated artifacts. Avoid committing `output/*.ppm` unless explicitly requested.

## Architecture Notes
- The project follows Ports & Adapters (Hexagonal Architecture). Keep new algorithms behind the existing `FractalAlgorithm` and `ColourMap` ports to preserve separation.
