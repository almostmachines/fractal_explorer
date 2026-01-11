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

## Task Tracking and Management

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

### Quick Reference

```bash
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task|feature|bug --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push changes
```

### Workflow Pattern

1. **Start**: Run `bd ready` to find actionable work.
2. **Claim**: Use `bd update <id> --status=in_progress`.
3. **Work**: Implement the issue and test.
4. **Discovery**: If you discover new work, create a new bead with discovered-from:<parent-id>.
5. **Complete**: Mark the issue as closed with `bd close <id>`. Commit and push your work.
6. **Sync**: Always run `bd sync` after marking an issue as complete, even if you didn't commit any work.

### Landing the Plane (Session Completion)

When ending a work session, complete all steps below.

1. **File issues for remaining work** - Create issues for anything that needs follow-up.
2. **Run quality gates** (if code changed) - Tests, linters, builds.
3. **Update issue status** - Close finished work, update in-progress items.
4. **Push to remote:**
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Verify** - All changes committed AND pushed.
6. **Hand off** - Provide context for next session.

### Best Practices

- Check `bd ready` at session start to find available work.
- Update status as you work (in_progress → closed).
- Create new issues with `bd create` when you discover tasks.
- Use descriptive titles and set appropriate priority/type.
- Always `bd sync` before ending session.

## Tips for Claude

### Generating tree diagrams

You can use the command `tree --gitignore`.
