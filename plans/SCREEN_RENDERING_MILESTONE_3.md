# Milestone 3 Plan — Cancellation and Coalescing (Cooperative)

## Context

Milestone 2 delivered an end-to-end pipeline that renders a Mandelbrot frame into the GUI window:

- `src/input/gui/app.rs` builds `RenderRequest`s from `UiState`, submits requests to the controller, and **filters** incoming frames by generation.
- `src/controllers/interactive/controller.rs` runs a worker thread, **coalesces** requests into a single latest slot, and uses a **generation id** to suppress stale frames (currently “soft cancellation”: stale work is discarded only after the render completes).
- `src/adapters/present/pixels_presenter.rs` stores the latest event and wakes the UI thread.

### What Milestone 3 adds

Milestone 3 upgrades “soft cancellation” to **cooperative cancellation**:

- When a new request supersedes an in-flight render, the old render should **stop quickly**, not continue consuming CPU until completion.
- Coalescing semantics (“latest request wins”) must remain intact.
- Stale frames must still **never** display.

This milestone is driven by the `plans/SCREEN_RENDERING_SPEC.md` requirements:

- “Implement render cancellation when inputs change.”
- “Coalescing always.”
- “Cancellation checks at least per row/tile or every N pixels.”
- “Do not emit RenderEvent::Error for cancellations.”

---

## Goals

### Functional

1. **Stale frames never show** (already implemented; must remain true):
   - Controller never emits frames for superseded generations.
   - UI ignores any `RenderEvent` where `event.generation != latest_submitted_generation`.

2. **Cooperative cancellation** (new):
   - When a new request arrives, old renders stop quickly (target: typically < 100ms).
   - Cancellation is treated as expected control flow, not an error visible to the user.
   - Closing the window (`shutdown`) cancels any in-flight render quickly so shutdown does not hang on long renders.

### Architectural

- Keep `src/core/` GUI-agnostic.
- Keep cancellation mechanism generic:
  - Core actions accept a cancellation token (e.g., `CancelToken`) or callback.
  - No `winit` / `pixels` / `egui` types or references outside `input/gui` and `adapters/present`.

---

## Scope decisions

### In scope

- Cooperative cancellation for:
  - fractal iteration generation (the heavy CPU step)
  - pixel buffer generation (colour mapping)
- Preserve existing “coalesce always” semantics in `InteractiveController`.
- Keep full-frame rendering (no progressive tiles/scanlines).

### Explicitly out of scope (Milestone 4+)

- Debounce/throttle UI input.
- Pan/zoom controls.
- Render-scale slider.
- Status/FPS panel (beyond what’s already present).
- Additional fractals and colour schemes.

---

## Implementation plan

### 1) Define cancellation semantics and types

**Definition:** a render job is *cancelled* when its cancellation token reports cancellation.

For `InteractiveController`, the cancellation condition should be:

- `shared.shutdown.load(Ordering::Relaxed) || job_gen != shared.generation.load(Ordering::Relaxed)`

**Recommended ordering:** use `Ordering::Relaxed` for cancellation polling (no data dependency; we just need a fast “has it changed?” check). This applies equally to checking the shutdown flag.

**Proposed new types (core-agnostic):**

- `Cancelled` marker type (`struct Cancelled;`) used as an explicit, allocation-free control-flow signal.
- `CancelToken` trait for cheap cancellation polling inside tight loops (avoid `&dyn Fn() -> bool` vtable calls in hot loops; keep it `Send + Sync` so it can be shared across rayon tasks):
  - Prefer passing `cancel: &impl CancelToken` (static dispatch).
  - Provide a `NeverCancel` token so non-cancelable code paths can share the same implementation.

Illustrative API (exact naming up to implementation):

```rust
pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

pub struct NeverCancel;

impl CancelToken for NeverCancel {
    fn is_cancelled(&self) -> bool { false }
}

// Blanket impl so callers can pass closures directly.
impl<F: Fn() -> bool + Send + Sync> CancelToken for F {
    fn is_cancelled(&self) -> bool { (self)() }
}
```

Where these live:

- Prefer `src/core/actions/` for cancellation-related utilities used by multiple actions.
  - Example file: `src/core/actions/cancellation.rs` exported by `src/core/actions/mod.rs`.

Deliverable:

- A small, reusable cancellation primitive that can be used by `generate_fractal*` and `generate_pixel_buffer*` without referencing GUI/controller types.
- Clear semantics: cancellation is expected control flow and must not be turned into a user-visible `RenderEvent::Error`.

---

### 2) Add a cancelable fractal generation action (rayon)

Problem today:

- `src/core/actions/generate_fractal/generate_fractal_parallel_rayon.rs` builds a `Vec<Point>` for all pixels up front and cannot stop mid-compute.

Target:

- A cancelable version that:
  - avoids allocating a `Vec<Point>`
  - checks cancellation at a predictable granularity
  - short-circuits quickly using rayon `try_*` APIs + `Result` propagation (cooperative early-exit)


**Recommended API (cancel-aware implementation + wrappers):**

- File: `src/core/actions/generate_fractal/generate_fractal_parallel_rayon.rs`
- Add a cancel-aware internal implementation (single source of truth), then expose thin wrappers:
  - `generate_fractal_parallel_rayon_cancelable(pixel_rect, algorithm, cancel: &impl CancelToken)`
  - refactor existing `generate_fractal_parallel_rayon(pixel_rect, algorithm)` to call the same implementation with `NeverCancel`

**Return type (cancel-aware/internal):**

- `Result<Vec<Alg::Success>, GenerateFractalError<Alg::Failure>>`

Example error enum (illustrative):

- `Cancelled`
- `Algorithm(Alg::Failure)`

**Cancellation granularity:**

- Check cancellation at a bounded granularity:
  - at least every `CANCEL_CHECK_INTERVAL_PIXELS` pixels *within a row* (i.e. based on `x`, so `x == 0` guarantees at least one check per row)
- Suggested constant: `CANCEL_CHECK_INTERVAL_PIXELS = 256` (power-of-two; define once in `core/actions/cancellation` and reuse across actions).

**Implementation approach (recommended for correctness + ordering):**

- Iterate rows in parallel (indexed range, preserves order when collected):
  - `row_index in 0..height`
- Each row computes pixels left→right:
  - check `cancel.is_cancelled()` every `CANCEL_CHECK_INTERVAL_PIXELS` pixels (based on `x`, so this includes `x == 0` and guarantees at least one check per row)
  - call `algorithm.compute(Point { x, y })`
  - push results into a row `Vec<Success>`
- Collect rows into `Vec<Vec<Success>>` (in row order) and then flatten.
- On cancel, return `Err(Cancelled)`;
  - controller treats this as expected and emits no `RenderEvent::Error`.

**Notes:**

- Prefer rayon `try_*` combinators (e.g. `try_for_each` / `try_fold` + `try_reduce`, or `collect::<Result<_, _>>()`) so `Err(Cancelled)` propagates promptly.
- Rayon may still execute some in-flight tasks after cancellation is detected; this is acceptable.
- The goal is to bound “time wasted on obsolete work” by chunk size.
- Optional optimization: replace `Vec<Vec<Success>>` with a single preallocated output buffer and fill per-row slices in parallel to reduce per-row allocations.

**Unit tests (core, GUI-free):**

Add tests that:

1. **Correctness (non-cancelled):** cancelable/shared implementation matches `generate_fractal_serial` output for a stub algorithm.
2. **Cancellation behavior:** a cancellation token that flips to “cancelled” after K polls causes the function to return `Err(Cancelled)`.
3. **Polling granularity:** verify `cancel.is_cancelled()` is polled at least once per row (assert `polls >= height`), and on a wide row (`width > CANCEL_CHECK_INTERVAL_PIXELS`) that polling happens more than once per row (avoid asserting exact counts).

---

### 3) Add a cancelable pixel buffer generation action

Problem today:

- `src/core/actions/generate_pixel_buffer/generate_pixel_buffer.rs` does a full `collect()` of colours and then flattens into bytes with no cancellation points.

Target:

- A cancelable version that:
  - checks cancellation periodically
  - avoids allocating an intermediate `Vec<Colour>`

**Recommended approach:**

- File: `src/core/actions/generate_pixel_buffer/generate_pixel_buffer.rs`
- Implement a cancel-aware internal function (single source of truth) that streams bytes directly into the output buffer (no intermediate `Vec<Colour>`), then expose thin wrappers:
  - `generate_pixel_buffer_cancelable(input, mapper, pixel_rect, cancel: &impl CancelToken)`
  - refactor existing `generate_pixel_buffer(input, mapper, pixel_rect)` to call the same implementation with `NeverCancel`

Implementation details:

- Preallocate `PixelBufferData` with `capacity = pixel_rect.size() * 3`.
- Iterate input values and:
  - check cancellation every `CANCEL_CHECK_INTERVAL_PIXELS` pixels (`cancel.is_cancelled()`), reusing the same constant as fractal generation
  - map value → `Colour { r, g, b }`
  - push `r, g, b` directly into the output byte buffer
- Build `PixelBuffer::from_data(pixel_rect, buffer)`

**Error handling:**

- Distinguish cancellation from “real errors”.
- Recommended: extend the existing `GeneratePixelBufferError` with a `Cancelled` variant, so cancelable and non-cancelable paths share one error type:
  - `Cancelled`
  - `ColourMap(err)`
  - `PixelBuffer(err)`

**Unit tests (core, GUI-free):**

1. Correctness of RGB output for a small input buffer.
2. Cancellation returns `Err(Cancelled)` and does not produce a `PixelBuffer`.

---

### 4) Wire cooperative cancellation into `InteractiveController`

Current state:

- `src/controllers/interactive/controller.rs` only checks generation *after* `render_request` finishes.

Target:

- Generation changes should be observed inside the compute loops.

**Controller changes:**

- In `worker_loop`, for each job generation `job_generation`:
  - create a cancellation token (a closure is fine; it can implement `CancelToken` via a blanket impl):
    - `cancel = || shared.shutdown.load(Ordering::Relaxed) || job_generation != shared.generation.load(Ordering::Relaxed)`
  - pass `&cancel` into the cancelable core actions.

**Render pipeline changes:**

- Update `render_request` (or a new internal helper) to:
  1. Validate the request as today.
  2. Call cancelable fractal generation:
     - if `Cancelled`, return “cancelled” outcome (do not emit error)
  3. Before colour mapping, re-check `cancel.is_cancelled()` (cheap fast-path)
  4. Call cancelable pixel buffer generation:
     - if `Cancelled`, return “cancelled” outcome

Implementation notes:

- Keep cancellation distinct from “real errors” all the way back to `worker_loop` (avoid turning `Cancelled` into a `String` via `.to_string()`/`format!`).
- Audit `GenerateFractalError`/`GeneratePixelBufferError` matches to handle `Cancelled` explicitly and avoid emitting `RenderEvent::Error`.
- A simple pattern is to have `render_request` return an internal enum like `RenderOutcome::{Rendered, Cancelled, Error(String)}`.

**Event emission rules:**

- If cancelled:
  - emit nothing (no `RenderEvent::Error`)
  - if shutdown is requested, exit promptly; otherwise loop back immediately to pick up the latest request

- If success:
  - still validate `job_generation == shared.generation.load(...)` before emitting (belt-and-suspenders)

- If real error:
  - emit `RenderEvent::Error` only when the job generation is still current

---

### 5) Tests for cancellation and coalescing semantics

Because “responsive within 100ms” is hard to test reliably with wall-clock timing, Milestone 3 should emphasize deterministic tests around:

- correct cancellation wiring
- correct suppression (no stale frames/errors)

**Recommended test layers:**

1. **Core-action tests (deterministic):**
   - cancellation tokens are polled
   - cancellation returns `Cancelled`

2. **Controller tests (semideterministic, keep small):**
   - Verify that cancellation does *not* emit `RenderEvent::Error`.
   - Verify stale `RenderEvent::Error` events are dropped when the generation mismatches.
   - Verify that after submitting A then B, the controller eventually emits a frame for B, and does not emit a frame for A.

3. **(Optional) Deterministic controller cancellation via test hooks:**
   - If timeout-based tests become flaky on CI, add minimal `#[cfg(test)]` hooks/latches around key phases in `InteractiveController` so tests can guarantee “A is in-flight” before submitting B.
   - Keep hooks fully compiled out in non-test builds to avoid runtime overhead.

To reduce flakiness, prefer:

- small render sizes (e.g., 64×64)
- stub algorithms/colour maps in core-action tests
- controller tests that assert “eventually” with a bounded timeout and do not assert exact timings
- when timeouts are unavoidable, make them generous (CI variance) and keep render sizes tiny so renders finish quickly even on slow machines

---

## Manual verification checklist

Run:

- `cargo test`
- `cargo run --features gui --bin gui`

Interactive checks:

1. Drag the “Max iterations” slider rapidly:
   - the UI remains responsive
   - CPU does not remain pegged rendering obsolete generations for long
   - the displayed frame tracks the most recently submitted generation

2. Resize the window continuously:
   - stale frames never show
   - rendering cancels and restarts promptly

3. Close the window while a long render is in-flight (e.g., high iterations):
   - the app exits promptly (shutdown does not wait for the obsolete render to finish)

---

## Risks / pitfalls

- **Rayon cancellation is cooperative:** some in-flight tasks may complete after cancellation. The plan relies on frequent cancellation checks to bound wasted work.
- **Granularity too coarse:** checking only once per row may be too slow at high resolutions/iterations. Prefer “every N pixels” checks.
- **Order of results:** if implementing via parallel row computation, ensure final flattened output is row-major to match existing mapping expectations.
- **Avoid up-front allocations:** prefer a single shared implementation that does not build `Vec<Point>`/`Vec<Colour>` up front (so both cancelable and non-cancelable paths benefit).

---

## Acceptance criteria (Milestone 3)

Milestone 3 is complete when:

- Stale frames still never display (generation filtering remains correct in UI and controller).
- Cooperative cancellation is implemented:
  - superseded renders stop early (bounded by cancellation granularity)
  - cancellation does not surface as `RenderEvent::Error`
  - shutdown does not hang on long renders (closing the window cancels in-flight work promptly)
- `cargo test` passes without `--features gui`.
- `cargo run --features gui --bin gui` remains functional and interactive under rapid input changes.

---

## Reference

- `plans/SCREEN_RENDERING_SPEC.md` — canonical architecture + milestone definitions
- `plans/SCREEN_RENDERING_MILESTONE_2.md` — current implemented baseline
