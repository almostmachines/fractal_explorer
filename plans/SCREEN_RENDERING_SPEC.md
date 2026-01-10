# Interactive Fractal Explorer (GUI) — Specification

## Summary

This document specifies the next major step for `fractal_explorer`: rendering fractals directly to the screen while accepting interactive user input (algorithm selection, colour scheme, coordinates, resolution, iterations, etc.). The GUI stack will use:

- `winit` for windowing and event loop
- `egui` for UI/widgets
- `pixels` for presenting the rendered image to the window surface

GUI dependencies should be optional behind a `gui` Cargo feature so core/CLI builds and tests remain lightweight.

The implementation preserves the existing ports & adapters approach:

- The fractal math, algorithms, and data types remain in `src/core/` (domain).
- A new interactive controller (application layer) orchestrates rendering jobs.
- A GUI input adapter (winit + egui) produces `RenderRequest`s.
- A presentation adapter implements an output port trait and bridges computed frames to the rendering library.

Cancellation is required: when new inputs arrive, in-flight rendering must be cancelled (or at minimum, superseded without presenting stale frames).

---

## Goals

### Functional

- Render fractals to a window in real time.
- Provide interactive controls for:
  - fractal algorithm selection (starting with Mandelbrot)
  - colour scheme selection
  - view coordinates (e.g. `ComplexRect` for Mandelbrot)
  - max iterations (and other algorithm-specific parameters)
  - render resolution (derived from window size and/or a user-controlled scale factor)
- Provide smooth interaction:
  - resizing the window updates the render resolution
  - changing parameters triggers a re-render
  - stale renders never appear onscreen
- Implement render cancellation when inputs change.

### Architectural

- Keep `src/core/` GUI-agnostic and reusable from both CLI and GUI.
- Keep “input” (winit/egui) separate from controllers and core logic.
- Use explicit ports (traits) for outputs (presentation), enabling future adapters (softbuffer, file output, etc.).
- Ensure rendering backends are isolated to adapters (no `pixels`, `winit`, or `egui` in core).

---

## Non-Goals (initial scope)

- GPU fractal generation (compute shaders).
- Persistent project/session management.
- Multi-window UI.
- High-end navigation tooling (bookmarks, orbit/animations) beyond basic pan/zoom.
- Networking features.

These may be added later without violating the architecture in this spec.

---

## Terminology

- **PixelRect**: Rectangle of pixels describing the render target. In this codebase, `PixelRect` uses *inclusive* bounds: `x ∈ [left, right]` and `y ∈ [top, bottom]` (so `width = right-left+1`, `height = bottom-top+1`). This matches how `core/actions/generate_fractal/*` iterate pixels and how `PixelBuffer` sizes its backing store.
- **ComplexRect**: Rectangle in the complex plane describing the view (e.g. Mandelbrot region). Coordinate convention note: `pixel_to_complex_coords` maps increasing screen `y` to increasing `imag`, so `ComplexRect.top_left().imag` corresponds to the top row and `ComplexRect.bottom_right().imag` to the bottom row.
- **Coordinate conventions**: Pixel-space origin is top-left; `x` increases right and `y` increases downward. (This is why “top_left” in `ComplexRect` corresponds to the top of the rendered image, even though many mathematical descriptions treat positive imaginary as “up”.)
- **RenderRequest**: Immutable set of parameters required to produce a frame.
- **Frame**: Rendered pixel data ready to be presented (in this codebase currently `PixelBuffer` is RGB).
- **Generation**: Monotonic identifier used to cancel/supersede older renders.

---

## High-Level Architecture (Ports & Adapters)

### Overview

```
           ┌──────────────────────────────┐
           │        input/gui (adapter)   │
           │ winit event loop + egui UI   │
           └──────────────┬───────────────┘
                          │ RenderRequest
                          ▼
           ┌──────────────────────────────┐
           │ controllers/interactive      │
           │  - owns job lifecycle        │
           │  - cancellation (generation) │
           │  - calls core actions        │
           └──────────────┬───────────────┘
                          │ RenderEvent via FrameSink port
                          ▼
           ┌──────────────────────────────┐
           │ adapters/present (adapter)   │
           │  FrameSink impl: store latest│
           │  + wake UI (EventLoopProxy)  │
           └──────────────┬───────────────┘
                          │ drained on UI thread
                          ▼
           ┌──────────────────────────────┐
           │ pixels + wgpu (presentation) │
           └──────────────────────────────┘
```

Key constraint: the window surface, `pixels`, and `wgpu` objects must be used from the UI/event-loop thread. Therefore, the controller/worker thread(s) only produce data and signal the UI; they do not directly call `pixels.render()`.

---

## Module Layout (proposed)

### New modules

```
src/
├── bin/
│   └── gui.rs                        # GUI entry point (separate binary)
├── controllers/
│   ├── mandelbrot.rs                 # existing CLI controller (PPM output)
│   └── interactive/
│       ├── mod.rs
│       ├── controller.rs             # InteractiveController (job mgmt)
│       ├── types.rs                  # RenderRequest + enums
│       └── ports/
│           ├── mod.rs
│           └── frame_sink.rs         # FrameSink output port
├── input/
│   └── gui/
│       ├── mod.rs                    # run_gui()
│       ├── app.rs                    # winit loop, ties everything together
│       ├── ui_state.rs               # state + change detection
│       └── panels/
│           ├── mod.rs
│           ├── universal.rs          # algorithm + resolution + common
│           └── mandelbrot.rs         # Mandelbrot-specific controls
└── adapters/
    └── present/
        ├── mod.rs
        └── pixels_presenter.rs       # FrameSink impl + latest-frame storage
```

### Existing modules remain

- `src/core/`: algorithms, data types, actions (generation + colour mapping)
- `src/storage/`: file output adapter (`write_ppm`)

---

## Public API surface (library)

### `src/lib.rs`

Continue exporting the CLI controller, and add (behind a feature flag) the GUI runner:

- `pub use controllers::mandelbrot::mandelbrot_controller;`
- `pub use input::gui::run_gui;` (likely behind `#[cfg(feature = "gui")]`)

Rationale: keep CLI-only builds lightweight; GUI dependencies can be optional.

### Cargo feature gating (recommended)

To ensure `cargo build`, `cargo test`, and the existing CLI (`cargo run`) stay fast and do not pull in `wgpu`/windowing dependencies by default:

- Add a `gui` feature in `Cargo.toml` that enables optional GUI dependencies (`winit`, `pixels`, `egui`, `egui-winit`, `egui-wgpu`, etc.).
- Mark the GUI binary as `required-features = ["gui"]` so it is only compiled when explicitly requested.
- Prefer running via: `cargo run --features gui --bin gui`.
- When using a hybrid approach (GUI code in the library + a thin `gui` binary), gate GUI module declarations in `src/lib.rs` with `#[cfg(feature = "gui")]` so non-GUI builds never compile `input/gui` modules or pull optional deps.

---

## Data Model and Messages

### RenderRequest

`RenderRequest` is the sole input to the interactive controller. It should contain:

- Universal settings:
  - `pixel_rect: PixelRect`
  - `fractal: FractalKind`
  - `colour_scheme: ColourSchemeKind`
- Fractal-specific parameters:
  - `params: FractalParams`

Proposed types (`src/controllers/interactive/types.rs`):

```rust
pub struct RenderRequest {
    pub pixel_rect: PixelRect,
    pub fractal: FractalKind,
    pub params: FractalParams,
    pub colour_scheme: ColourSchemeKind,
}

pub enum FractalKind {
    Mandelbrot,
    // Julia,
    // BurningShip,
}

pub enum FractalParams {
    Mandelbrot {
        region: ComplexRect,
        max_iterations: u32,
    },
    // Julia { region: ComplexRect, c: Complex, max_iterations: u32 },
}

pub enum ColourSchemeKind {
    BlueWhiteGradient,
    // ...
}
```

The input adapter builds `RenderRequest` from UI state and passes it to the controller.

### Frame and output port

The domain already has `PixelBuffer` (RGB, 3 bytes/pixel). To minimize churn, the first iteration of the GUI will use `PixelBuffer` as the “frame” type and convert it to RGBA in the presentation adapter.

Output messages:

```rust
pub struct FrameMessage {
    pub generation: u64,
    pub pixel_rect: PixelRect, // redundant but convenient for validation
    pub pixel_buffer: PixelBuffer,
    pub render_duration: std::time::Duration,
}

pub struct RenderErrorMessage {
    pub generation: u64,
    pub message: String,
}

pub enum RenderEvent {
    Frame(FrameMessage),
    Error(RenderErrorMessage),
}
```

Output port:

```rust
pub trait FrameSink: Send + Sync {
    fn submit(&self, event: RenderEvent);
}
```

Requirements for `FrameSink::submit`:

- Must be thread-safe.
- Must not call `pixels.render()` or access wgpu surface resources directly.
- Should be non-blocking or minimally blocking (do not stall worker threads).
- Should wake the UI/event loop so frames appear promptly (preferred).

---

## Interactive Controller

### Responsibilities

The interactive controller is an application-layer orchestrator. It must:

- Accept render requests (from the input adapter).
- Manage cancellation/supersession of in-flight renders.
- Run CPU fractal generation and colour mapping using existing `core/actions`.
- Deliver render events to the output port (`FrameSink`).

### Threading model

- UI thread runs winit loop and presentation.
- Controller owns a worker thread (or a small worker pool) that performs compute.
- The worker thread may use `rayon` internally for parallelism.
  - Optional: run renders inside a controller-owned `rayon::ThreadPool` (e.g. `max(1, available_parallelism() - 1)` threads) to preserve CPU headroom for the UI thread. Keep CLI/global pool behaviour unchanged.
- At most one “active” render request is considered current; older ones are cancelled/superseded.

#### Request coalescing (“coalesce always”)

Because this spec chooses “coalesce always”, the controller should *not* enqueue an unbounded stream of render requests. Instead, keep a single “latest pending request” slot that gets overwritten:

- `latest_request: Mutex<Option<(u64, RenderRequest)>>`
- `wakeup: Condvar` (or equivalent)

On each new request, overwrite the slot and notify the worker. The worker always renders the most recent pending request once it becomes free.

#### Shutdown / lifecycle

Define a clean shutdown path:

- When the winit loop is exiting, signal the controller to stop (e.g. `shutdown: AtomicBool = true`) and join the worker thread.
- The presenter should treat wake failures after shutdown (e.g. `EventLoopProxy::send_event`) as non-fatal and simply drop the wake request.

### Cancellation model (generation-based)

Controller maintains:

- `generation: AtomicU64` (starts at 0)

On each new request:

- Increment generation and assign it to the render job.
- Send the job to the worker.

Worker behaviour:

- Periodically checks if the job generation is still current:
  - `if job_gen != controller.generation.load(...) { return; }`
- Only submits frames if still current.

UI behaviour:

- Tracks `latest_generation` and drops any `RenderEvent::Frame(msg)` where `msg.generation != latest_generation` (and similarly ignores stale `RenderEvent::Error(err)` events).

This ensures stale frames never display.

#### “True” cancellation vs “soft” cancellation

- **Soft cancellation**: old jobs continue computing but their frames are ignored.
  - Minimal code changes; acceptable as a first step.
  - CPU can still be saturated while user drags sliders.
- **Cooperative cancellation (required target)**: old jobs stop quickly.
  - Implemented by checking the current generation at a predictable granularity (per-row, per-tile, or per-N pixels).
  - Treat cancellation as an expected control-flow outcome (e.g. `Cancelled`) rather than a user-visible “error”.
  - Do not emit `RenderEvent::Error` for cancellations; cancellations are expected during interaction.
  - For rayon-based implementations, cancellation can be represented as an early `Err(Cancelled)` and propagated via `try_*` APIs to stop work promptly; some work may continue briefly due to in-flight tasks.
  - Avoid heavy up-front work for jobs that are likely to be cancelled (e.g. allocating a `Vec<Point>` of every pixel). Prefer iterating an indexed range (`0..pixel_count`) and deriving `(x,y)` from the index so cancellation can short-circuit earlier with less memory churn.

Acceptance criteria (target):

- When parameters change rapidly (slider drag), CPU usage should not remain pegged rendering obsolete frames for long; old renders should *typically* stop within a small bounded time once superseded (aim: < 100ms). To make this achievable, ensure the cancellation check granularity bounds the worst-case time (e.g. check at least once per row or every N pixels).

### Scheduling and debouncing

To avoid thrashing:

- The input adapter should not send a new render request for every minor event unless desired.
- Implement either:
  - **debounce** (e.g. schedule render N ms after last change), or
  - **coalescing** (only keep latest request; worker always renders the latest generation)

Recommended first approach:

- Coalescing + generation cancellation is sufficient.
- Add a simple debounce for high-frequency inputs (mouse wheel/drag) later if needed.

---

## Input Layer (GUI adapter)

### Responsibilities

The GUI input layer:

- Owns winit event loop and window.
- Owns egui context and UI state.
- Renders UI (egui) each frame.
- Converts UI state into a `RenderRequest` and calls the controller when changes occur.
- Consumes frames from the presentation adapter and presents them via `pixels`.

### UI state model

`UiState` is split into:

- universal state:
  - `fractal: FractalKind`
  - render scale / resolution settings
  - `colour_scheme: ColourSchemeKind`
- fractal-specific state (enum with per-fractal structs):
  - `FractalUiState::Mandelbrot(MandelbrotUiState)`

Each fractal panel is responsible for:

- Editing its own state via egui.
- Producing validated `FractalParams` (clamping, enforcing invariants).

### Change detection

To avoid redundant renders, the UI adapter should:

- Maintain the last submitted `RenderRequest` (or a hash of it).
- Only submit a new request if meaningful fields changed.

For ergonomics:

- Prefer to build a `RenderRequest` each frame, then compare with last request via `PartialEq` or a lightweight hash.

### Resize + DPI handling (important edge cases)

- Distinguish **surface size** (the winit window’s physical pixel size) from **render size** (the fractal buffer resolution; may be scaled down via a “render scale” setting).
- On winit resize / scale-factor-change events:
  - always resize the `pixels` *surface* to match the new physical window size;
  - only submit a new `RenderRequest` (and/or resize the `pixels` *buffer*) when the chosen render size is valid (`width >= 2` and `height >= 2`).
- If the window is minimized (0×0) or otherwise too small, pause rendering and keep the last valid frame (do not treat this as an error).

### Coordinate controls

Minimum viable set:

- Numeric inputs for `ComplexRect`:
  - min real, max real
  - min imag, max imag
- Optionally, preserve aspect ratio automatically.

Recommended interaction additions (still consistent with architecture):

- Pan: click-drag to translate the region
- Zoom: scroll wheel zoom around cursor

Implementation notes:

- Use the existing pixel-to-complex conversion utilities where possible.
- When the window size changes, adjust `ComplexRect` to match the pixel aspect ratio around the current center:
  - Keep center fixed
  - Expand width or height to match `pixel_aspect = width/height`
- If render resolution != window surface resolution, map cursor/window coordinates into render-buffer pixel coordinates before calling `pixel_to_complex_coords` (otherwise zoom/pan will be incorrect under scaling and DPI changes).

---

## Presentation Adapter (`pixels`)

### Responsibilities

The pixels adapter has two concerns:

1. **Output port implementation** (`FrameSink`):
   - Accept `RenderEvent` from controller/worker thread.
   - Store the latest frame (and generation) in a thread-safe slot (and optionally the latest error for UI display).
   - Wake the UI thread so it will redraw promptly.

2. **Presentation helper (UI thread)**:
   - Provide a method to obtain the latest frame for drawing.
   - Convert RGB `PixelBuffer` (3 bytes/pixel) into the `pixels` frame format (RGBA8, 4 bytes/pixel).
   - Handle window resize by resizing the `Pixels` surface and/or internal frame size.

Recommended structure:

- `PixelsPresenter` struct used by `input/gui/app.rs`:
  - `FrameSink` implementation for the controller.
  - UI-thread methods:
    - `take_latest()` or `latest_ref()` to get the newest frame
    - `copy_into_pixels_frame(&mut Pixels)` to copy RGB→RGBA efficiently

### Frame format conversion

`PixelBuffer` is RGB; `pixels` expects RGBA8.

- Conversion strategy:
  - Write `r,g,b,255` per pixel.
  - Optionally premultiply alpha later if needed (not required for opaque fractal).

Performance considerations:

- Conversion should avoid allocations per frame.
- Store a reusable RGBA scratch buffer sized to the current `PixelRect`.

---

## Rendering Pipeline

### Steps (per render job)

1. Input adapter emits `RenderRequest { pixel_rect, fractal, params, colour_scheme }`.
2. Controller selects algorithm + colour map based on enums.
3. Controller invokes:
   - fractal iteration generation (existing `core/actions/generate_fractal/*`)
   - pixel buffer generation (existing `core/actions/generate_pixel_buffer`)
4. Controller wraps the result as `FrameMessage { generation, pixel_rect, pixel_buffer, render_duration }`.
5. Controller delivers `RenderEvent::Frame(...)` via `FrameSink`.
6. UI thread receives wake event, pulls the latest frame, converts to RGBA, and calls `pixels.render()`.
7. Egui UI renders on top (or alongside) depending on chosen integration.

### Egui + pixels integration strategy

`pixels` presents via wgpu. Egui rendering also needs a renderer (commonly `egui-wgpu`).

Chosen implementation (Milestone 1):

1. **Overlay UI on top of fractal in the same window** (preferred and selected):
   - On each redraw:
     - update pixels frame
     - render pixels to surface
     - render egui paint jobs via wgpu on the same surface (after pixels)
   - Implementation recommendation (keeps all wgpu/egui details inside `input/gui`):
     - Use `egui-winit` to translate winit events → `egui::RawInput`.
     - Use `egui-wgpu` to render egui paint jobs.
     - Reuse the `wgpu::Device`, `wgpu::Queue`, and render target format from `pixels` (so there is exactly one wgpu surface).
     - Compose by running the egui render pass *after* the pixels pass using whichever “custom pass / extra render pass” hook the chosen `pixels` version exposes.
   - Note: ensure `pixels` and `egui-wgpu` use the same `wgpu` major version to share device/queue types.

Alternative options (if needed later):

2. **Side-by-side UI layout**:
   - Render fractal into the pixels frame and draw UI in a separate region (still in same surface).
   - Still requires egui integration; region management is in UI layer, not controller.

3. **Fallback (if integration is temporarily blocked)**:
   - First milestone draws fractal only, with keyboard/mouse shortcuts (no egui).
   - Then add egui once rendering pipeline is stable.

The chosen option keeps egui fully inside the `input/gui` adapter (no egui types leaking into controller/core).

---

## Error Handling

### Principles

- GUI should not crash on recoverable errors (invalid params, transient resize issues).
- Errors should surface to the user in the UI (status bar / toast / debug panel).

### Controller errors

Controller should convert domain errors into an application error type:

- invalid request (e.g. `PixelRect` invalid, iterations == 0)
- algorithm compute errors (rare if requests are validated)
- colour mapping errors

For UI friendliness:

- errors should be converted to a displayable form (string or structured enum) and delivered to the UI via `RenderEvent::Error`.
- do not treat cancellation (`Cancelled`) as an error event; it is expected during interaction.
- errors must not poison the rendering loop; after an error, another request should still work.

---

## Testing Strategy

### Unit tests

- `controllers/interactive`:
  - generation-based cancellation: stale frames are dropped
  - controller selects correct algorithm/colour map given enums
  - request validation and clamping
- `adapters/present`:
  - RGB→RGBA conversion produces correct output for a small buffer
  - “latest frame wins” semantics

### Integration tests (optional)

- Headless tests are limited due to winit window requirements.
- Prefer testing the message passing + rendering pipeline with mocked `FrameSink` and without opening a window.

---

## Implementation Plan (milestones)

### Milestone 1 — compile-ready GUI skeleton

- Add `src/bin/gui.rs` that launches a window (winit) and runs an event loop.
- Add a `gui` Cargo feature + optional GUI dependencies + `required-features` gating for the `gui` binary.
- Ensure `pixels` and `egui-wgpu` share the same `wgpu` major version.
- Gate GUI module declarations in `src/lib.rs` (not just `pub use`) so non-GUI builds never compile GUI code.
- Add pixels surface and render a placeholder pattern.
- Handle resize + scale-factor-change events using physical sizes when resizing the `pixels` surface.
- Add minimal egui panel (even a basic “Hello” + a slider), rendered via a custom pass after the pixels pass.

### Milestone 2 — controller + ports

- Add `controllers/interactive` with:
  - `RenderRequest` types
  - `FrameSink` port (accepts `RenderEvent` frames/errors)
  - worker thread that can render Mandelbrot using existing core actions
- Wire GUI input → controller → presenter → pixels.

### Milestone 3 — cancellation and coalescing

- Add generation IDs and ensure stale frames never display.
- Implement cooperative cancellation (tile/row chunking + generation checks).

### Milestone 4 — UX improvements

- Add pan/zoom controls.
- Add render-scale slider and resize behavior.
- Add status panel: render time, current generation, current FPS.

### Milestone 5 — extensibility

- Add a second fractal (e.g. Julia) to validate fractal-specific panels and params enum.
- Add a second colour scheme.

---

## Open Questions / Decisions

1. **PixelBuffer format**
   - Keep RGB in domain and convert in adapters (current spec), or migrate to RGBA in core?
   - RGB keeps core small and matches PPM output; conversion cost is acceptable initially.

2. **Aspect ratio handling**
   - Preserve complex region (stretch image) vs adjust complex region to match window aspect ratio.
   - Default recommendation: adjust region to match aspect ratio around current center.

3. **Progressive rendering**
   - Full-frame only initially, or stream scanlines/tiles for responsiveness?
   - Tile-based rendering improves perceived performance and makes cancellation more responsive.

4. **Render scheduling**
   - Always render on any change vs “render on mouse-up” for some controls.
   - Recommended: coalesce always; optional debounce for high-frequency updates.

### Answers received:

1. **PixelBuffer format**
    - Keep as RGB in the core.

2. **Aspect ratio handling**
    - Adjust region as this will be the expected behaviour.

3. **Progressive rendering**
    - Use full-frame rendering.

4. **Render scheduling**
   - Coalesce always; optional debounce for high-frequency updates.

---

## Acceptance Criteria

- `cargo build` / `cargo test` without `--features gui` stays lightweight (GUI deps and the `gui` binary are feature-gated).
- `cargo run --features gui --bin gui` opens a window and displays a Mandelbrot render.
- Changing any of:
  - iterations,
  - colour scheme,
  - region bounds,
  - window size / render scale
  triggers a new render.
- Stale frames never show after new inputs are applied.
- Rendering work cancels/supersedes quickly enough to keep UI responsive (cancellation checks at least per row / tile or every N pixels).
