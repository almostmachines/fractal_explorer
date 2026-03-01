# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build              # Debug build
cargo build --release    # Optimized build
cargo test               # Run all tests (118 total)
cargo test --lib         # Run unit tests only
cargo test <test_name>   # Run specific test
cargo run                # Generate fractal to output/mandelbrot.ppm
```

**GUI (requires `gui` feature):**
```bash
cargo run --release --features gui --bin gui  # Launch interactive GUI
cargo build --features gui                    # Build with GUI support
```

## Task Tracking and Management

This project uses **br** (beads) for issue tracking. Run `br --help` to get started.

### Quick Reference

```bash
br ready              # Show issues ready to work (no blockers)
br list --status=open # All open issues
br show <id>          # Full issue details with dependencies
br create --title="..." --type=task|feature|bug --priority=2
br update <id> --status=in_progress
br close <id> --reason="Completed"
br close <id1> <id2>  # Close multiple issues at once
br sync               # Commit and push changes
```

### Workflow Pattern

1. **Start**: Run `br ready` to find actionable work.
2. **Claim**: Use `br update <id> --status=in_progress`.
3. **Work**: Implement the issue and test.
4. **Discovery**: If you discover new work, create a new bead with discovered-from:<parent-id>.
5. **Complete**: Mark the issue as closed with `br close <id>`. Commit and push your work.
6. **Sync**: Always run `br sync` after marking an issue as complete, even if you didn't commit any work.
7. **Repeat**: Repeat this workflow pattern until there are no more beads to work on

### Landing the Plane (Session Completion)

When ending a work session, complete all steps below.

1. **File issues for remaining work** - Create issues for anything that needs follow-up.
2. **Run quality gates** (if code changed) - Tests, linters, builds.
3. **Update issue status** - Close finished work, update in-progress items.
4. **Push to remote:**
   ```bash
   git pull --rebase
   br sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Verify** - All changes committed AND pushed.
6. **Hand off** - Provide context for next session.

### Best Practices

- Check `br ready` at session start to find available work.
- Update status as you work (in_progress → closed).
- Create new issues with `br create` when you discover tasks.
- Use descriptive titles and set appropriate priority/type.
- Always `br sync` before ending session.

## Tips for Claude

### Generating tree diagrams

You can use the command `tree --gitignore`.

### GUI Visual Testing

You can visually test the GUI by taking screenshots with `grim` (Wayland).

```bash
# Build and run GUI in background
cargo run --bin gui --features gui &
GUI_PID=$!

# Wait for window to render
sleep 2

# Capture screenshot
grim /tmp/fractal_gui_test.png

# Kill the GUI
kill $GUI_PID

# View the screenshot using the Read tool on /tmp/fractal_gui_test.png
```

**Limitations:**
- Snapshot-based, not real-time
- Cannot interact with GUI (clicking, dragging)
- May need timing adjustments for slower renders

## Project

A Rust-based Mandelbrot fractal renderer with both CLI and interactive GUI capabilities. Features parallel rendering, multiple colour maps, and real-time exploration.

## Architecture

The codebase follows **hexagonal architecture** (ports & adapters). See `ARCHITECTURE.md` for full details.

### Layer Overview

```
src/
├── core/           # Pure domain logic (no external deps)
│   ├── data/       # Complex, PixelBuffer, Colour, rects
│   ├── fractals/   # Mandelbrot algorithm + colour maps
│   └── actions/    # Use cases: generate_fractal, generate_pixel_buffer
├── controllers/    # Application orchestration
│   ├── cli/                 # CLI (synchronous)
│   └── interactive/         # GUI (async with worker thread)
├── input/gui/      # winit + egui event handling
├── presenters/     # wgpu framebuffer rendering
└── storage/        # PPM file output
```

### Key Ports (Traits)

- **`FractalAlgorithm`**: Computes iteration count per pixel
- **`ColourMap<T>`**: Maps iteration counts to RGB colours
- **`CancelToken`**: Cooperative cancellation (checked every 1024 pixels)
- **`InteractiveControllerPresenterPort`**: Receives rendered frames for display

### Rendering Pipeline

```
PixelRect + Algorithm → Vec<u32> iterations → PixelBuffer RGB → Output
```

### Adding a New Colour Map

1. Add variant to `MandelbrotColourMapKinds` in `core/fractals/mandelbrot/colour_mapping/kinds.rs`
2. Create implementation in `core/fractals/mandelbrot/colour_mapping/maps/`
3. Register in `mandelbrot_colour_map_factory()` in `factory.rs`

### GUI Threading Model

- Main thread: UI rendering (egui/winit)
- Worker thread: Fractal computation
- Generation IDs track request versions; stale results are discarded
- Request coalescing: new requests replace pending ones
