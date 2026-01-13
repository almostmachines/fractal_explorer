# Milestone 2 Plan — Controller + Ports (Mandelbrot to Screen)

## Context

Milestone 1 established a feature-gated GUI skeleton:

- `src/bin/gui.rs` exists and runs only with `--features gui`
- `src/input/gui/app.rs` opens a winit window
- `pixels` renders a placeholder pattern
- `egui` overlays a small debug panel

Milestone 2 replaces the placeholder renderer with a real Mandelbrot render generated via existing `src/core/` actions, while preserving the ports & adapters architecture described in `plans/SCREEN_RENDERING_SPEC.md`.

### Architectural goal (ports & adapters)

Milestone 2 introduces the application/controller layer and the output port/presenter adapter:

```
input/gui (adapter)         controllers/interactive            adapters/present
(winit + egui UI)   ->      (job orchestration)        ->      (FrameSink + pixels bridge)
  builds RenderRequest          runs worker thread                 stores latest frame
  triggers renders              calls core actions                 wakes UI thread

UI thread: owns pixels + wgpu surface and calls pixels.render().
Worker thread: computes PixelBuffer only.
```

## Objectives

1. Add `controllers/interactive`:
   - `RenderRequest` input types (Mandelbrot-only for now)
   - `FrameSink` output port and `RenderEvent` message types
   - an `InteractiveController` that renders frames on a worker thread using existing core actions

2. Add a presentation adapter:
   - `FrameSink` implementation that stores the latest frame/error
   - a UI-thread helper that copies RGB `PixelBuffer` into the RGBA `pixels` frame
   - a wake mechanism so new frames redraw promptly

3. Wire GUI → controller → presenter → pixels:
   - GUI converts UI state into a `RenderRequest`
   - GUI submits requests only when meaningfully changed
   - on wake/redraw, GUI presents the latest completed frame

4. Ensure stale frames never display (spec compliance via “soft cancellation”):
   - each request gets a monotonically increasing generation id
   - renders may still run to completion, but results from superseded generations are discarded

## Scope decisions (Milestone 2)

### In-scope

- Mandelbrot only (`FractalKind::Mandelbrot`)
- One colour scheme only (`ColourSchemeKind::BlueWhiteGradient`)
- Full-frame rendering (no progressive tiles)
- Background rendering on a dedicated worker thread
- Request coalescing via a single “latest request wins” slot (overwrite semantics)
- Generation ids + stale-frame suppression (“soft cancellation”)
- Minimal UI controls required to demonstrate rerendering (iterations + region bounds are enough)

### Explicitly deferred to Milestone 3+

- Cooperative cancellation (row/tile checks inside core loops to stop work early)
- Debounce/throttle options for high-frequency input (optional)
- Pan/zoom interaction, aspect-ratio correction, render-scale slider, status panel

Milestone 2 implements *generation-based supersession* (discarding stale results) because it materially improves correctness/UX while remaining relatively simple. The spec introduces generation IDs in Milestone 3; this plan intentionally pulls that piece forward so the first on-screen Mandelbrot already obeys “stale frames never show”. Milestone 3 then focuses on *stopping* obsolete work quickly (cooperative cancellation) and any additional scheduling refinements.

---

## Implementation Tasks

### 1) Add `controllers/interactive` module skeleton

**Files to add**

- `src/controllers/interactive/mod.rs`
- `src/controllers/interactive/types.rs`
- `src/controllers/interactive/controller.rs`
- `src/controllers/interactive/ports/mod.rs`
- `src/controllers/interactive/ports/frame_sink.rs`

**Files to modify**

- `src/controllers/mod.rs` (export new module)

**Types to define (`src/controllers/interactive/types.rs`)**

- `RenderRequest` (immutable “render this frame” input)
- `FractalKind` (start with `Mandelbrot`)
- `FractalParams` (start with `Mandelbrot { region: ComplexRect, max_iterations: u32 }`)
- `ColourSchemeKind` (start with `BlueWhiteGradient`)

Recommended derives for ergonomic change-detection and debugging:

- `#[derive(Debug, Clone, PartialEq)]` for `RenderRequest`, `FractalParams`
- `#[derive(Debug, Copy, Clone, PartialEq, Eq)]` for the `*Kind` enums

**Output messages (`src/controllers/interactive/ports/frame_sink.rs`)**

- `FrameMessage` (contains at least `generation`, `pixel_rect` and `PixelBuffer`; include render duration)
- `RenderErrorMessage` (contains at least `generation`; string message is fine initially)
- `RenderEvent` enum (`Frame` | `Error`)
- `FrameSink` trait:
  - `pub trait FrameSink: Send + Sync { fn submit(&self, event: RenderEvent); }`

Notes:

- Keep this module GUI-agnostic: no `winit`, `pixels`, `egui`, `wgpu` types.
- Use domain types where possible: `PixelRect`, `ComplexRect`, `PixelBuffer`.

### 2) Implement `InteractiveController` with a worker thread

**File**

- `src/controllers/interactive/controller.rs`

**Public API (suggested)**

- `InteractiveController::new(frame_sink: Arc<dyn FrameSink>) -> Self`
- `InteractiveController::submit_request(&self, request: RenderRequest) -> u64` (returns the request generation)
- `InteractiveController::shutdown(self)` (or `Drop` joins worker thread)

**Threading model (Milestone 2)**

- The controller owns:
  - one worker thread
  - a `generation: AtomicU64` (monotonic request id)
  - a `latest_request` slot (e.g., `Mutex<Option<(u64, RenderRequest)>>`)
  - a wake primitive (`Condvar` or channel)
  - a shutdown signal (`AtomicBool`)

Worker loop behavior:

1. Wait until either a request exists or shutdown is signaled.
2. Take the most recent `(generation, request)` (overwrite semantics allowed).
3. Render the request by calling existing core actions:
   - Build algorithm: `MandelbrotAlgorithm::new(pixel_rect, region, max_iterations)`
   - Generate fractal iterations: `generate_fractal_parallel_rayon(pixel_rect, &algorithm)`
   - Generate pixels: `generate_pixel_buffer(fractal, &MandelbrotBlueWhiteGradient::new(max_iterations), pixel_rect)`
4. Before emitting, confirm the job is still current (“soft cancellation”):
   - if `job_generation != generation.load(...)`, discard the result and loop
5. Submit either:
   - `RenderEvent::Frame(FrameMessage { generation: job_generation, pixel_rect, pixel_buffer, render_duration, ... })`, or
   - `RenderEvent::Error(RenderErrorMessage { generation: job_generation, message, ... })`

Validation expectations:

- Do not submit frames for invalid window sizes:
  - GUI should avoid creating invalid `PixelRect`, but controller may still defensively validate.
- Clamp or reject invalid inputs (pick one approach and document it):
  - `max_iterations == 0` should become a UI validation issue; controller can return an error event.

**Important non-goals for Milestone 2**

- No *cooperative* cancellation. If a request is superseded while rendering, it is acceptable that the earlier job completes.
- However, results from superseded generations must be discarded so stale frames never display (“soft cancellation”).

**Rayon pool note (recommended for GUI responsiveness)**

`generate_fractal_parallel_rayon` uses Rayon’s global pool internally. In a GUI app, consider creating a controller-owned `rayon::ThreadPool` with fewer threads (e.g., `max(1, available_parallelism().saturating_sub(2))`) and running the render via `pool.install(|| ...)` so the UI thread reliably has CPU headroom.

Tune the thread count as needed; the goal is to reserve CPU for the UI thread.

### 3) Add `adapters/present` pixels presenter + `FrameSink` implementation

Milestone 2 needs a bridge from the worker-produced `PixelBuffer` to the UI thread `Pixels` framebuffer.

**Files to add**

- `src/adapters/mod.rs` (NOT feature-gated; gate only GUI-dependent submodules)
- `src/adapters/pixel_format.rs` (NOT feature-gated; pure RGB/RGBA helpers + tests)
- `src/adapters/present/mod.rs` (feature-gated via parent module)
- `src/adapters/present/pixels_presenter.rs` (feature-gated via parent module)

Module gating detail (to keep `cargo test` without `--features gui` lightweight):

- In `src/adapters/mod.rs`: `pub mod pixel_format;` and `#[cfg(feature = "gui")] pub mod present;`.
- Avoid any `use pixels::...` / `use winit::...` outside `#[cfg(feature = "gui")]` blocks.

**Files to modify**

- `src/lib.rs` (declare `mod adapters;` unconditionally; ensure only GUI-dependent submodules pull GUI deps)

**Presenter responsibilities**

1. Implement `FrameSink`:
   - store the latest `RenderEvent` in a thread-safe slot (e.g., `Mutex<Option<RenderEvent>>`)
   - store the latest error separately if useful for UI
   - wake the UI/event loop to prompt a redraw

2. UI-thread helper methods:
   - `take_latest_event()` (drain semantics; “latest wins”)
   - `copy_pixel_buffer_into_pixels_frame(pixel_buffer: &PixelBuffer, pixels: &mut Pixels)`
     - converts RGB (3 bytes/pixel) → RGBA8 (4 bytes/pixel, alpha=255)
     - **no allocations**: write directly into `pixels.frame_mut()` using `chunks_exact_mut(4)` zipped with the source `chunks_exact(3)`
     - keep the conversion logic in a pure helper (e.g., `adapters::pixel_format`) so it can be unit-tested without `--features gui`


**Wake mechanism (recommended)**

- Switch the GUI `EventLoop` to a user-event type (e.g. `enum GuiEvent { Wake }`).
- The presenter stores a `winit::event_loop::EventLoopProxy<GuiEvent>`.
- On `FrameSink::submit`, after storing the latest event:
  - call `proxy.send_event(GuiEvent::Wake)`
  - ignore send failures (e.g., window already closing)

Notes:

- The `FrameSink::submit` implementation must not touch `Pixels`, `wgpu`, or the window surface.
- UI-thread methods may take `&mut Pixels` and perform the copy.

### 4) Wire `input/gui` to the controller + presenter

**Primary file to modify**

- `src/input/gui/app.rs`

**File to add (recommended, aligns with spec layout)**

- `src/input/gui/ui_state.rs` (GUI state + RenderRequest construction + change detection)

**4.1 Introduce a `GuiEvent` user event**

- Change `EventLoop<()>` → `EventLoop<GuiEvent>`
- Create an `EventLoopProxy<GuiEvent>`
- Handle `Event::UserEvent(GuiEvent::Wake)` by ensuring a redraw is requested (e.g. call `window.request_redraw()`, or set `redraw_pending = true` and request redraw in `MainEventsCleared`).
- Keep this explicit: receiving a user event does not by itself guarantee `RedrawRequested` will fire.

**4.2 Instantiate presenter + controller during app startup**

- Create a `PixelsPresenter` (or similar) with the proxy.
- Create an `InteractiveController` with `presenter.frame_sink()` (likely `Arc<dyn FrameSink>`).
- Store both on the `App` struct.

**4.3 Replace placeholder drawing with “present latest frame”**

In `RedrawRequested`:

1. Run egui frame (`update_ui`) as today.
2. Drain presenter for latest render event.
3. If a new frame exists, its `generation` matches the most recently submitted request (final correctness gate), and its size matches the current pixels buffer:
   - copy RGB → RGBA into `pixels.frame_mut()`.
4. Call `pixels.render_with(...)` exactly as today (pixels scaling pass, then egui pass).

**4.4 Add minimal GUI state → RenderRequest construction**

To demonstrate the full pipeline, add a minimal `UiState` module (recommended: `src/input/gui/ui_state.rs`, matching the spec layout):

- Mandelbrot region bounds (defaults to classic view)
- `max_iterations` (default 256)

In the egui debug panel, add:

- a slider for `max_iterations` (clamp to >= 1)
- numeric inputs for region bounds (min/max real + min/max imag) OR a “Reset view” button + fixed region for Milestone 2

Each redraw (or after UI interaction):

- Compute a candidate `RenderRequest`:
  - `pixel_rect` from current render resolution (initially: window physical size)
  - `fractal = FractalKind::Mandelbrot`
  - `params = FractalParams::Mandelbrot { region, max_iterations }`
  - `colour_scheme = ColourSchemeKind::BlueWhiteGradient`

- Compare against `last_submitted_request: Option<RenderRequest>`.
  - If changed, submit request to controller and store the returned generation (for stale-frame suppression).

Implementation note:
- Prefer keeping `UiState` responsible for producing a validated `RenderRequest` (or returning “no request” if inputs are invalid), so `app.rs` remains mostly event-loop + wiring glue.

**4.5 Resize handling and request submission**

On `WindowEvent::Resized` and `ScaleFactorChanged`:

- Always resize the `pixels` *surface* to match the new physical window size (and continue using physical sizes on DPI/scale-factor changes).
- Only resize the `pixels` *buffer* and submit a new `RenderRequest` when the chosen render size is valid (`width >= 2 && height >= 2`).
- If the window is minimized (0×0) or too small, pause rendering and keep the last valid frame (do not surface an error).
- Mark `redraw_pending = true` (and ensure it results in `window.request_redraw()` as noted above).

**4.6 UI error/status display**

- If the presenter stores the last error message, display it in the egui panel only when it matches the most recently submitted generation (ignore stale errors).
- Also show:
  - current generation (most recently submitted)
  - last render duration (if available)
  - current render resolution

**4.7 Shutdown / close handling**

- On `WindowEvent::CloseRequested`, shut down the controller so the worker thread is joined (via an explicit `shutdown()` call or `Drop`).
- Treat `EventLoopProxy::send_event` failures during/after shutdown as expected (window closing) and ignore them.

### 5) Add unit tests for the new layers

Milestone 2 adds concurrency and conversion logic; tests should focus on deterministic, GUI-free pieces.

**Controller tests (no `gui` feature required)**

- A `MockFrameSink` that stores submitted events (e.g., `Mutex<Vec<RenderEvent>>` or a channel).
- Submit a small request:
  - `PixelRect` 2×2 or 4×4
  - classic Mandelbrot region
  - small `max_iterations` (e.g., 10)
- Assert:
  - a `RenderEvent::Frame` is received within a reasonable timeout (keep render size tiny to avoid flaky CI)
  - `pixel_buffer.buffer().len() == width*height*3`
  - `pixel_rect` matches
  - `generation` exists and is stable per request

- Add a supersession/stale-drop test (deterministic):
  - submit request A, record `g1`; submit request B, record `g2`; assert `g2 > g1`
  - test the final UI/presenter gate: when “current generation = g2”, any frame/error with `generation = g1` is ignored
  - if testing the controller directly, add a test-only latch/barrier so request A is guaranteed in-flight when B is submitted, then assert no post-supersession submit uses `generation = g1`

**Presenter conversion tests**

- Preferred: keep RGB→RGBA conversion in a pure helper function (no `pixels` dependency) in a non-feature-gated module (e.g., `src/adapters/pixel_format.rs`) and test it in `cargo test` without `--features gui`.
- Optional: add an integration-style test behind `#[cfg(feature = "gui")]` to validate wiring into `Pixels` if desired.

### 6) Verification / manual run checklist

- `cargo build`
- `cargo test`
- `cargo run --features gui --bin gui`
  - window opens
  - Mandelbrot image appears (not placeholder)
  - adjusting iterations triggers a new render
  - resizing the window triggers a new render (at the new resolution)

---

## Notes / Pitfalls to watch

- **PixelRect is inclusive**: bottom-right should be `(width-1, height-1)`.
- **Minimum sizes**: `PixelRect::new` requires width/height >= 2; handle 0×0 minimize and tiny sizes.
- **Type conversions**: window size is `u32`, but `PixelRect` uses `i32`; use safe conversions.
- **Thread boundaries**: never touch `Pixels` / `wgpu` / surface objects off the UI thread.
- **Backpressure**: without cooperative cancellation, avoid spamming render requests:
  - only submit when `RenderRequest` changed
  - consider “Apply” button if edits are too chatty (optional)

---

## Reference

For the full architecture and later milestones (cancellation, coalescing, pan/zoom), see [SCREEN_RENDERING_SPEC.md](./SCREEN_RENDERING_SPEC.md).

---

## Acceptance Criteria (Milestone 2)

- `cargo build` and `cargo test` succeed without `--features gui`, and GUI deps remain feature-gated.
- `cargo run --features gui --bin gui` displays an actual Mandelbrot render generated via `src/core/` (no placeholder pattern).
- GUI updates trigger rerenders:
  - changing `max_iterations` in the egui panel submits a new `RenderRequest` and eventually updates the displayed image.
  - resizing the window resizes the pixels surface/buffer and results in a new render at the new resolution.
- Rendering runs off the UI thread (window remains responsive while renders compute).
- Stale frames never display: UI ignores any frame/error where `generation != latest_submitted_generation` (controller may also drop stale results).
- A `FrameSink` port exists under `src/controllers/interactive/ports/` and is used by the controller to deliver `RenderEvent`s.
- The presentation adapter stores “latest frame” and safely converts RGB `PixelBuffer` → RGBA `pixels` frame without per-frame allocations.
- Basic errors (invalid params) surface in the UI (via `RenderEvent::Error`) rather than crashing the app.
