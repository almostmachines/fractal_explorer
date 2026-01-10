# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build              # Debug build
cargo build --release    # Optimized build
cargo test               # Run all tests (87 total)
cargo test --lib         # Run unit tests only
cargo test <test_name>   # Run specific test
cargo run                # Generate fractal to output/mandelbrot.ppm
```

## Architecture Overview

This is a Mandelbrot set renderer that generates PPM images. The architecture uses a ports & adapters pattern with trait-based abstractions.

### Data Flow Pipeline

```
mandelbrot_controller (src/controllers/mandelbrot.rs)
  │
  ├─► generate_fractal_parallel_rayon() → Vec<u32> iteration counts
  │     Uses FractalAlgorithm trait (MandelbrotAlgorithm impl)
  │
  ├─► generate_pixel_buffer() → PixelBuffer RGB data
  │     Uses ColourMap trait (BlueWhiteGradient impl)
  │
  └─► write_ppm() → output/mandelbrot.ppm
```

### Key Traits (Ports)

- **FractalAlgorithm** (`src/core/actions/generate_fractal/ports/`): Defines fractal computation interface. Implement to add new fractal types.
- **ColourMap** (`src/core/actions/generate_pixel_buffer/ports/`): Defines colour mapping interface. Implement to add new colour schemes.

### Module Structure

- `controllers/` - Orchestration logic
- `core/data/` - Data types: Complex, Point, Colour, PixelRect, ComplexRect, PixelBuffer
- `core/fractals/mandelbrot/` - Mandelbrot algorithm and colour maps
- `core/actions/generate_fractal/` - Three parallel + one serial fractal generation implementations
- `core/actions/generate_pixel_buffer/` - Iteration count to RGB conversion
- `core/util/` - Coordinate conversion utilities
- `storage/` - PPM file output

### Parallel Implementations

Three parallel strategies exist in `src/core/actions/generate_fractal/`:
- `generate_fractal_parallel_rayon.rs` - **Current default**, uses Rayon work-stealing
- `generate_fractal_parallel_scoped_threads.rs` - Uses `thread::scope()`
- `generate_fractal_parallel_arc.rs` - Uses `Arc<T>` with manual thread management

All produce identical results (verified by tests). Non-default implementations are marked with `#[allow(dead_code)]`.

### Current Configuration

Hardcoded in `mandelbrot_controller()`:
- Image: 800x600 pixels
- Complex plane: real [-2.5, 1.0], imaginary [-1.0, 1.0]
- Max iterations: 256

## Task tracking

- Use 'bd' for task tracking

### Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Tips for Claude

### Generating tree diagrams

You can use the command `tree --gitignore`.
