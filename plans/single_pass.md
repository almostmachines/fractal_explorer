# Single-Pass Rendering Plan (Compute + Colour Map Together)

## Goal

Eliminate the current two-stage render pipeline:

1. **Fractal compute** (parallel): `PixelRect` -> `Vec<u32>` iteration/escape values
2. **Colour mapping** (single-threaded): `Vec<u32>` -> `PixelBuffer` (RGBA bytes)

Replace it with a **single render action** that produces a `PixelBuffer` directly, doing **compute + map in one pass**, and doing the mapping work **in the same parallel pass** as computation.

Success criteria:

- No full-frame intermediate `Vec<u32>` allocation in the common path.
- Colour mapping is parallelized (no single-threaded second pass).
- Output `PixelBuffer` is byte-for-byte identical to the old pipeline for the same algorithm + colour map.
- Cancellation and error propagation remain correct for the interactive controller.
- `InteractiveController` continues propagating cancellation as `RenderOutcome::Cancelled` (not as an error message).

## Current Implementation (What We’re Changing)

- Compute stage:
  - `src/core/actions/generate_fractal/generate_fractal_parallel_rayon.rs`:
    - per-row parallel compute via `FractalAlgorithm::compute_row_segment_into(...)`
    - returns `Vec<Alg::Success>` (for Mandelbrot/Julia: `Vec<u32>`)
- Map stage:
  - `src/core/actions/generate_pixel_buffer/generate_pixel_buffer.rs`:
    - iterates the `Vec<T>` sequentially
    - calls `ColourMap<T>::map(T) -> Result<Colour, Box<dyn Error>>`
    - builds `PixelBufferData` (RGBA) then `PixelBuffer::from_data_opaque(...)`

The second stage is currently a bottleneck because it:

- is single-threaded
- traverses the entire pixel set again
- requires holding a full-frame `Vec<u32>` in memory

## High-Level Design

Introduce a new core action that renders a `PixelBuffer` in one go:

- Module: `src/core/actions/render_pixel_buffer/`
- API (rayon-based, matching current usage):
  - `render_pixel_buffer_parallel_rayon(pixel_rect, algorithm, colour_map) -> Result<PixelBuffer, RenderPixelBufferError<AlgErr>>`
  - `render_pixel_buffer_parallel_rayon_cancelable(pixel_rect, algorithm, colour_map, cancel) -> Result<PixelBuffer, RenderPixelBufferCancelableError<AlgErr>>`

Then update call sites:

- CLI path: `CliTestController::generate()` should call the single-pass renderer.
- Interactive path: `InteractiveController::render_request()` should call the cancelable single-pass renderer.
- Benches: add a “single_pass” benchmark alongside the current “generate_and_map”.

## Key Technical Decisions

### 1) Keep `FractalAlgorithm` unchanged (for now)

`FractalAlgorithm::compute_row_segment_into(...)` is already optimized in `MandelbrotAlgorithm` and `JuliaAlgorithm` (step-based coordinate progression inside bounds). We want to preserve that optimization.

To keep the trait object usage in `FractalConfig` working, we avoid adding generic/closure-based methods to `FractalAlgorithm` (those would break object safety).

### 2) Make colour-map errors `Send + Sync` so rayon can propagate them

To do mapping inside rayon worker threads and return the first error cleanly, the error type flowing through rayon must be `Send`.

Today `ColourMap::map(...)` returns `Box<dyn Error>` (not `Send`), which prevents using it in a parallel iterator.

Plan:

- Introduce a shared error alias in `src/core/actions/generate_pixel_buffer/ports/colour_map.rs`:
  - `pub type ColourMapError = Box<dyn std::error::Error + Send + Sync + 'static>;`
- Update `ColourMap<T>` to:
  - `fn map(&self, value: T) -> Result<Colour, ColourMapError>;`
- Update all colour map implementations + tests accordingly.

This is a small, contained API change (only a handful of implementations exist).

### 3) Parallelize over rows and write directly into the final `PixelBufferData`

Implementation approach:

- Allocate the final output buffer once:
  - `let mut buffer = vec![0u8; pixel_rect.size() as usize * 4];`
- Fill it in parallel by row:
  - split `buffer` into `par_chunks_mut(row_bytes)` where `row_bytes = pixel_rect.width() as usize * 4`
  - `enumerate()` to map row index -> actual y coordinate: `y = top_left_y + row_idx as i32`
  - for each row:
    - compute iterations for the row via `algorithm.compute_row_segment_into(y, x_start, x_end, &mut iters_row)`
    - map each `u32` to `Colour` and write `r,g,b,a` into the row slice

This avoids:

- locking
- atomic per-pixel coordination
- a second pass across the whole image

It also keeps output ordering deterministic (each row corresponds to a fixed slice).

### 4) Cancellation checks

The single-pass renderer should preserve current semantics:

- cancellation is expected control flow, not an “error message”
- cancellation should be checked frequently enough (using `CANCEL_CHECK_INTERVAL_PIXELS`)

In the combined row worker:

- check `cancel.is_cancelled()` at row start
- keep the existing chunk loop pattern (CANCEL_CHECK_INTERVAL_PIXELS-sized x-segments)
- check `cancel.is_cancelled()` between segments (same granularity as `generate_fractal_parallel_rayon`)
- optionally add an extra check while mapping if we want even tighter responsiveness

Note: rayon cannot instantly stop all workers, but returning an error from `try_for_each` will short-circuit collection and other workers will see cancellation checks soon after.

## Proposed API and Error Types

New error enums (in the new action module) should be explicit about the failure source:

- `RenderPixelBufferError<AlgErr>`
  - `Algorithm(AlgErr)`
  - `ColourMap(ColourMapError)`
  - `PixelBuffer(PixelBufferError)` (should be unreachable if we size correctly, but keep for safety)

- `RenderPixelBufferCancelableError<AlgErr>`
  - `Cancelled(Cancelled)`
  - `Algorithm(AlgErr)`
  - `ColourMap(ColourMapError)`
  - `PixelBuffer(PixelBufferError)`

Keep `Display` messages consistent with existing conventions:

- “algorithm error: …”
- “colour map error: …”
- “pixel buffer error: …”

## Step-by-Step Implementation Plan

### Step 1: Update `ColourMap` error type for parallel use

Files:

- `src/core/actions/generate_pixel_buffer/ports/colour_map.rs`
- All concrete colour maps:
  - `src/core/fractals/mandelbrot/colour_mapping/maps/*.rs`
  - `src/core/fractals/julia/colour_mapping/maps/*.rs`
  - boxed delegators in `.../colour_mapping/map.rs`
- `src/core/actions/generate_pixel_buffer/generate_pixel_buffer.rs` (error enums + tests)

Work:

- Replace `Box<dyn Error>` with `ColourMapError` everywhere.
- Ensure tests still downcast (`err.downcast_ref::<...>()`) successfully.

Quality gate:

- `cargo test` (no GUI features required)

### Step 2: Add the single-pass renderer action (rayon)

Files:

- `src/core/actions/mod.rs` (export new module)
- `src/core/actions/render_pixel_buffer/mod.rs`
- `src/core/actions/render_pixel_buffer/render_pixel_buffer_parallel_rayon.rs` (or a single file)

Implementation sketch:

```rust
use rayon::prelude::*;
use crate::core::actions::cancellation::{CancelToken, Cancelled, NeverCancel, CANCEL_CHECK_INTERVAL_PIXELS};
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;
use crate::core::actions::generate_pixel_buffer::ports::colour_map::{ColourMap, ColourMapError};
use crate::core::data::pixel_buffer::{PixelBuffer, PixelBufferData, PixelBufferError};
use crate::core::data::pixel_rect::PixelRect;

pub fn render_pixel_buffer_parallel_rayon<Alg, CMap>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
) -> Result<PixelBuffer, RenderPixelBufferError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
{
    render_pixel_buffer_parallel_rayon_cancelable_impl(pixel_rect, algorithm, colour_map, &NeverCancel)
        .map_err(|e| match e {
            RenderPixelBufferCancelableError::Cancelled(_) => unreachable!(),
            RenderPixelBufferCancelableError::Algorithm(e) => RenderPixelBufferError::Algorithm(e),
            RenderPixelBufferCancelableError::ColourMap(e) => RenderPixelBufferError::ColourMap(e),
            RenderPixelBufferCancelableError::PixelBuffer(e) => RenderPixelBufferError::PixelBuffer(e),
        })
}

pub fn render_pixel_buffer_parallel_rayon_cancelable<Alg, CMap, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
    cancel: &C,
) -> Result<PixelBuffer, RenderPixelBufferCancelableError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
    C: CancelToken,
{
    render_pixel_buffer_parallel_rayon_cancelable_impl(pixel_rect, algorithm, colour_map, cancel)
}

fn render_pixel_buffer_parallel_rayon_cancelable_impl<Alg, CMap, C>(
    pixel_rect: PixelRect,
    algorithm: &Alg,
    colour_map: &CMap,
    cancel: &C,
) -> Result<PixelBuffer, RenderPixelBufferCancelableError<Alg::Failure>>
where
    Alg: FractalAlgorithm<Success = u32> + Sync + ?Sized,
    Alg::Failure: Send,
    CMap: ColourMap<u32> + ?Sized,
    C: CancelToken,
{
    let width = pixel_rect.width() as usize;
    let height = pixel_rect.height() as usize;
    let row_bytes = width * PixelBuffer::BYTES_PER_PIXEL;
    let x_start = pixel_rect.top_left().x;
    let x_end = pixel_rect.bottom_right().x;
    let top_y = pixel_rect.top_left().y;

    let mut buffer: PixelBufferData = vec![0u8; width * height * PixelBuffer::BYTES_PER_PIXEL];

    buffer
        .par_chunks_mut(row_bytes)
        .enumerate()
        .try_for_each(|(row_idx, row)| -> Result<(), RenderPixelBufferCancelableError<Alg::Failure>> {
            if cancel.is_cancelled() {
                return Err(RenderPixelBufferCancelableError::Cancelled(Cancelled));
            }

            let y = top_y + row_idx as i32;
            let mut chunk_start = x_start;
            let mut iters = Vec::with_capacity(CANCEL_CHECK_INTERVAL_PIXELS as usize);

            while chunk_start <= x_end {
                if cancel.is_cancelled() {
                    return Err(RenderPixelBufferCancelableError::Cancelled(Cancelled));
                }

                let chunk_end = chunk_start
                    .saturating_add(CANCEL_CHECK_INTERVAL_PIXELS as i32 - 1)
                    .min(x_end);

                iters.clear();
                algorithm
                    .compute_row_segment_into(y, chunk_start, chunk_end, &mut iters)
                    .map_err(RenderPixelBufferCancelableError::Algorithm)?;

                for (offset, iter) in iters.iter().enumerate() {
                    let c = colour_map
                        .map(*iter)
                        .map_err(RenderPixelBufferCancelableError::ColourMap)?;
                    let base =
                        ((chunk_start - x_start) as usize + offset) * PixelBuffer::BYTES_PER_PIXEL;
                    row[base] = c.r;
                    row[base + 1] = c.g;
                    row[base + 2] = c.b;
                    row[base + 3] = PixelBuffer::ALPHA_OPAQUE;
                }

                chunk_start = chunk_end + 1;
            }
            Ok(())
        })?;

    PixelBuffer::from_data_opaque(pixel_rect, buffer)
        .map_err(RenderPixelBufferCancelableError::PixelBuffer)
}
```

Notes:

- We constrain `Alg::Success = u32` because all current fractals use `u32`. If we want future generality, we can genericize later, but keep the first iteration focused.
- Chunk-local `Vec<u32>` keeps algorithm-side optimizations (step-based coordinate mapping) while preserving the current cancellation granularity.

Quality gates:

- `cargo test`

### Step 3: Switch CLI + interactive controllers to the new action

Files:

- `src/controllers/cli/test/cli_test.rs`
- `src/controllers/interactive/controller.rs`

Work:

- Replace:
  - `generate_fractal_parallel_rayon(...)` + `generate_pixel_buffer(...)`
  - `generate_fractal_parallel_rayon_cancelable(...)` + `generate_pixel_buffer_cancelable(...)`
- With:
  - `render_pixel_buffer_parallel_rayon(...)`
  - `render_pixel_buffer_parallel_rayon_cancelable(...)`

Keep error mapping behavior in `InteractiveController::render_request()`:

- cancellation -> `RenderOutcome::Cancelled`
- any other error -> `RenderOutcome::Error(err.to_string())`

Notes:

- Keep typed error variants (`Algorithm`, `ColourMap`, `PixelBuffer`, `Cancelled`) inside the new action API.
- Convert to `RenderOutcome::Error(String)` only at the existing controller boundary, so external error granularity does not regress relative to current behavior.
- Preserve current cancellation semantics: cancelled work should be dropped silently (no `RenderEvent::Error`).

Quality gates:

- `cargo test`
- `cargo run` (CLI should still emit `output/mandelbrot.ppm`)

### Step 4: Update benchmarks to measure the improvement

File:

- `benches/render_pipeline.rs`

Work:

- Add a new benchmark in `bench_full_pipeline`:
  - `BenchmarkId::new("single_pass", params.label)`
  - body calls `render_pixel_buffer_parallel_rayon(...)`
- Keep existing “generate_and_map” benchmark for baseline comparison.
- Ensure the comparison is apples-to-apples by running both variants with the same per-scenario inputs and harness settings:
  - same `SCENARIOS` entry (`width`, `height`, `max_iterations`, complex viewport)
  - same colour map kind/config for a given scenario
  - same throughput reporting and Criterion group configuration
  - same per-`params.label` loop structure so each result pair is directly comparable

Expected outcome:

- Lower total render time due to:
  - removing the full-frame `Vec<u32>` allocation
  - removing serial colour mapping
  - reduced cache misses / improved locality (write final bytes once)

### Step 5: Add/extend tests for behavioral equivalence

Add tests in the new action module:

- Determinism / correctness:
  - Use a stub algorithm producing a predictable sequence (e.g. `x + y`), and a stub colour map mapping `u32 -> Colour`.
  - Assert the resulting `PixelBufferData` matches what the old two-pass pipeline would produce.
- Error propagation:
  - Colour map failure is returned as `ColourMap(...)`.
  - Algorithm failure is returned as `Algorithm(...)`.
- Cancellation:
  - A cancel token that flips true mid-render triggers `Cancelled`.

This protects against regressions when we later optimize further (e.g. removing the row-local `Vec<u32>`).

### Step 6: Optional cleanup (after we’re confident)

Options (do later, not required for the initial merge):

1. Keep `generate_fractal*` + `generate_pixel_buffer` as reusable building blocks (useful for experimentation and focused benches).
2. Mark the old two-pass “pipeline” in controllers as removed; update `ARCHITECTURE.md` to describe the new single-pass default.
3. Consider adding a purely serial single-pass function for environments without rayon (if needed).

## Follow-Up Optimizations (Not Required Day 1)

If we want to remove the per-row `Vec<u32>` allocation entirely (true compute+map interleaving), we’ll need to overcome the current object-safety constraints around `FractalAlgorithm`:

- Either:
  - avoid trait objects in `FractalConfig` and dispatch on the enum variants directly, enabling generic/closure-based compute methods, or
  - introduce an object-safe “row writer” port (with careful attention to per-pixel dynamic dispatch costs).

Given the current architecture and limited trait-object usage, the enum-dispatch approach is likely the cleanest if we decide the extra optimization is worth it.
