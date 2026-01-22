# Repository Guidelines

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
│   ├── mandelbrot.rs        # CLI (synchronous)
│   └── interactive/         # GUI (async with worker thread)
├── input/gui/      # winit + egui event handling
├── presenters/     # wgpu framebuffer rendering
└── storage/        # PPM file output
```

### Key Ports (Traits)

- **`FractalAlgorithm`**: Computes iteration count per pixel
- **`ColourMap<T>`**: Maps iteration counts to RGB colours
- **`CancelToken`**: Cooperative cancellation (checked every 1024 pixels)
- **`PresenterPort`**: Receives rendered frames for display

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

## GUI Run Permissions
- When running the GUI app (for example `cargo run --features gui --bin gui`), **always request escalated permissions**.
  - Reason: the sandbox seccomp profile blocks Wayland socket connections, which triggers `WaylandError(Connection(NoCompositor))` even when Wayland is available.
  - Alternative: run the session in `danger-full-access` mode so GUI commands can execute without needing per-command escalation.

## GUI Visual Testing
- visually test the GUI by taking screenshots with `grim` (Wayland).

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

## Coding Style & Naming Conventions
- Follow standard Rust style (rustfmt defaults, 4-space indentation).
- Use `snake_case` for functions/modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines
- Tests live next to the code under `#[cfg(test)]` blocks.
- Prefer small, deterministic unit tests
- Name tests descriptively (for example `test_generate_fractal_returns_ok`).
- Run all tests with `cargo test` before opening a PR.

## Commit & Pull Request Guidelines
- Commit subjects are short, sentence-case, imperative (examples from history: `Rename fractal generation actions`).
- Keep commits scoped; avoid mixing refactors with feature changes.
- PRs should include: a concise summary, tests run, and sample output images or notes when rendering changes (`output/*.ppm`).

## Tips
- To generate tree diagrams you can use the command `tree --gitignore`.

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

**Important**. If you're working with beads tasks, you must follow this work flow.

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
