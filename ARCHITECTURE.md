# Architecture

This repository is a Mandelbrot fractal renderer with:

- A CLI “test generator” that writes a PPM file (`cargo run` → `output/mandelbrot.ppm`)
- An interactive GUI (feature-gated) for real-time exploration (`cargo run --bin gui --features gui`)

The codebase follows a **hexagonal architecture (ports & adapters)**: the domain logic is isolated in `src/core/`, while UI/IO live behind traits (“ports”) implemented by adapters in `src/presenters/` and `src/input/`.

## High-level layout

```
src/
├── core/                 # Pure computation + domain types (no UI, no filesystem)
│   ├── data/             # Complex numbers, rects, PixelBuffer, etc.
│   ├── fractals/         # Mandelbrot algorithm + colour mapping
│   ├── actions/          # Use-cases: generate_fractal, generate_pixel_buffer (+ cancellation)
│   └── util/             # Coordinate mapping, banding helpers
├── controllers/          # Application orchestration (CLI + interactive worker)
│   ├── cli/              # CLI flows (synchronous)
│   └── interactive/      # Interactive rendering controller (worker thread + events)
├── presenters/           # Output adapters
│   ├── file/             # File output (PPM)
│   └── pixels/           # GUI framebuffer presenter (pixels + egui) (feature = "gui")
├── input/                # Input adapters (feature-gated GUI)
│   └── gui/              # winit event loop + commands wiring (feature = "gui")
├── main.rs               # CLI entry point
└── bin/gui.rs             # GUI entry point (feature = "gui")
```

## Build targets and feature gates

- Library crate exports common entry points from `src/lib.rs`.
- CLI binary: `src/main.rs` (default features; generates a PPM).
- GUI binary: `src/bin/gui.rs` (`required-features = ["gui"]` in `Cargo.toml`).

GUI-only code is behind `cfg(feature = "gui")` (notably `src/input/` and `src/presenters/pixels/`).

## Core domain (“inside”)

The core is where the “what” lives: algorithms, data models, and the use-cases that transform them.

### Key domain types

- `PixelRect`, `Point` define image-space bounds (`src/core/data/`).
- `Complex`, `ComplexRect` define the complex-plane region (`src/core/data/`).
- `PixelBuffer` is the final RGB byte buffer with its `PixelRect` (`src/core/data/pixel_buffer.rs`).

### Use-cases (actions)

The main rendering pipeline is:

1. **Fractal iteration**: compute an iteration count per pixel (`Vec<u32>` for Mandelbrot)
2. **Colour mapping**: map iteration counts to RGB bytes (`PixelBuffer`)

Implementation lives in:

- `src/core/actions/generate_fractal/` (serial + parallel implementations)
- `src/core/actions/generate_pixel_buffer/`

Both have **cancel-aware** variants using `src/core/actions/cancellation.rs`. Cancellation is treated as expected control flow (do not surface as UI “errors”).

### Core ports (traits)

Core logic depends on traits rather than concrete UI/IO:

- `FractalAlgorithm` (`src/core/actions/generate_fractal/ports/fractal_algorithm.rs`): computes a value for a pixel.
- `ColourMap<T>` (`src/core/actions/generate_pixel_buffer/ports/colour_map.rs`): maps a computed value to an RGB colour.

## Controllers (“application”)

Controllers orchestrate the core to produce outputs.

### CLI flow

`src/main.rs` wires a controller + presenter:

1. `CliTestController::generate()` builds `PixelRect`, `ComplexRect`, `MandelbrotAlgorithm`
2. Runs `generate_fractal_parallel_rayon(...)`
3. Runs `generate_pixel_buffer(...)` with a chosen colour map
4. `CliTestController::write(...)` calls a `FilePresenterPort` adapter to write the file

The file-output port is `FilePresenterPort` (`src/controllers/ports/file_presenter.rs`), with a PPM adapter at `src/presenters/file/ppm.rs`.

### Interactive (GUI) flow

The interactive stack splits responsibilities:

- **UI thread**: winit/egui event loop (`src/input/gui/app/gui_app.rs`)
- **Worker thread**: compute frames and send results back (`src/controllers/interactive/controller.rs`)

The handshake looks like:

1. UI builds a `FractalConfig` based on window size + controls (`src/input/gui/app/state.rs`)
2. UI submits it to `InteractiveController::submit_request(...)` (returns a **generation id**)
3. Worker renders using cancel-aware actions and emits `RenderEvent` (frame or error)
4. A presenter adapter stores the latest event and wakes the event loop for redraw

The controller-to-presenter port is `InteractiveControllerPresenterPort` (`src/controllers/interactive/ports/presenter.rs`), implemented by `PixelsAdapter` (`src/presenters/pixels/adapter.rs`).

## Concurrency and cancellation

There are two layers of concurrency:

- **In-core parallelism**: `generate_fractal_parallel_rayon` uses rayon to parallelize rows.
- **GUI worker thread**: `InteractiveController` runs a single worker thread that always renders the *latest* submitted request.

The interactive controller uses:

- A monotonically increasing **generation counter** to identify requests and discard stale results.
- A “latest request wins” queue (`Mutex<Option<...>>`) to coalesce rapid UI updates.
- A cancellation token that returns `true` when:
  - the app is shutting down, or
  - a newer generation has been submitted.

## GUI Threading Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Thread                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   winit     │───▶│   GuiApp    │───▶│  PixelsPresenter    │  │
│  │ Event Loop  │    │  + egui     │    │  (wgpu rendering)   │  │
│  └─────────────┘    └──────┬──────┘    └─────────────────────┘  │
│                            │                      ▲              │
│                   submit_request()         GuiEvent::Wake        │
│                            │                      │              │
└────────────────────────────┼──────────────────────┼──────────────┘
                             ▼                      │
┌────────────────────────────────────────────────────────────────┐
│                       Worker Thread                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              InteractiveController                        │  │
│  │  - Receives FractalConfig requests                        │  │
│  │  - Runs fractal generation (cancelable)                   │  │
│  │  - Emits RenderEvent via PixelsAdapter                    │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Extension points

### Add a new Mandelbrot colour map

1. Implement `MandelbrotColourMap` (see `src/core/fractals/mandelbrot/colour_mapping/map.rs`).
2. Add a new variant to `MandelbrotColourMapKinds` (`src/core/fractals/mandelbrot/colour_mapping/kinds.rs`).
3. Register the implementation in `mandelbrot_colour_map_factory(...)` (`src/core/fractals/mandelbrot/colour_mapping/factory.rs`).

### Add a new file output format

1. Implement `FilePresenterPort` (`src/controllers/ports/file_presenter.rs`).
2. Wire it into a controller/binary (the CLI currently uses `PpmFilePresenter`).

### Add a new fractal type (beyond Mandelbrot)

At a minimum:

- Implement a new `FractalAlgorithm`.
- Add a new variant to `FractalConfig` (`src/controllers/interactive/data/fractal_config.rs`) and extend the interactive controller’s dispatch.
- Add UI controls for selecting/configuring it (`src/input/gui/app/state.rs` / `gui_app.rs`).

## Practical commands

```bash
cargo build
cargo run                        # writes output/mandelbrot.ppm
cargo test

cargo run --bin gui --features gui
```
