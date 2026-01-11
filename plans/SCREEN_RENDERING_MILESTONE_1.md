# Milestone 1 Plan — GUI Skeleton (compile-ready)

## Context

This milestone is the first of five in the **Interactive Fractal Explorer** project. The project transforms the existing CLI-based Mandelbrot renderer into a real-time interactive GUI application.

### Project Overview

The `fractal_explorer` codebase currently renders Mandelbrot fractals to PPM files via a CLI workflow. The goal is to add an interactive GUI that:

- Renders fractals directly to a window in real time
- Provides interactive controls for algorithm selection, colour schemes, view coordinates, max iterations, and resolution
- Supports smooth interaction with stale-frame prevention and render cancellation
- Preserves the existing ports & adapters architecture

### Technology Stack

The GUI will use:
- **winit** — windowing and event loop
- **egui** — immediate-mode UI widgets
- **pixels** — software rendering to a window surface (backed by wgpu)

All GUI dependencies are optional behind a `gui` Cargo feature so the CLI and core library remain lightweight.

### Architecture (Ports & Adapters)

The spec preserves the existing hexagonal architecture:

```
┌──────────────────────────────────────┐
│  input/gui (adapter)                 │  ← winit event loop + egui UI
│  Produces RenderRequest              │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│  controllers/interactive             │  ← Orchestrates render jobs
│  Owns worker thread, cancellation    │     Calls core actions
└──────────────┬───────────────────────┘
               │ RenderEvent via FrameSink port
               ▼
┌──────────────────────────────────────┐
│  adapters/present                    │  ← FrameSink impl, stores latest frame
│  Wakes UI thread                     │     RGB→RGBA conversion
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│  pixels + wgpu (presentation)        │  ← Renders to window surface
└──────────────────────────────────────┘
```

Key constraint: the window surface, `pixels`, and `wgpu` objects must be used from the UI/event-loop thread. Controller/worker threads only produce data and signal the UI.

### Milestone Roadmap

1. **Milestone 1 (this document)** — GUI skeleton: feature-gated build, window, pixels rendering, egui overlay
2. **Milestone 2** — Controller + ports: `InteractiveController`, `FrameSink`, worker thread, wire to pixels
3. **Milestone 3** — Cancellation and coalescing: generation IDs, cooperative cancellation, stale-frame prevention
4. **Milestone 4** — UX improvements: pan/zoom, render-scale slider, status panel
5. **Milestone 5** — Extensibility: additional fractals (Julia), additional colour schemes

---

## Objectives

This milestone establishes the GUI foundation without any fractal rendering logic. The objectives are:

1. **Feature-gated build** — Add a `gui` Cargo feature that enables all GUI dependencies. Without the feature, `cargo build` and `cargo test` must not pull in GUI code or dependencies.

2. **Window creation** — Open a resizable window using winit with a proper event loop.

3. **Pixels integration** — Initialize a `pixels` surface tied to the window and render a placeholder pattern (proving the render pipeline works).

4. **Egui overlay** — Render egui on top of the pixels frame using the same wgpu device/queue. Display a minimal panel (label + slider) to prove integration.

5. **Resize handling** — Handle window resize and DPI/scale-factor changes correctly, resizing both the pixels surface and buffer.

6. **Clean module structure** — Place all GUI code under `src/input/gui/` to match the ports & adapters layout. Gate module declarations (not just exports) so non-GUI builds never compile GUI code.

---

## Goal

Create a feature-gated GUI entry point that opens a window, renders a placeholder via `pixels`, and shows a minimal `egui` panel. The build must remain lightweight without `--features gui`.

## Tasks
1. **Cargo feature + deps**
   - Add a `gui` feature in `Cargo.toml`.
   - Add optional GUI deps (`winit`, `pixels`, `egui`, `egui-winit`, `egui-wgpu` or equivalent).
   - Mark the `gui` binary with `required-features = ["gui"]`.
   - Ensure `pixels` and `egui-wgpu` share the same `wgpu` major version.
   - Ensure the `winit` major version is consistent across `pixels`, `egui-winit`, and any direct `winit` dependency.

2. **Binary entry point**
   - Create `src/bin/gui.rs` that calls a GUI runner (e.g., `input::gui::run_gui()`).
   - Ensure it compiles only when `gui` is enabled.

3. **GUI module scaffold**
   - Add `src/input/gui/mod.rs` exposing `run_gui()`.
   - Gate GUI module declarations in `src/lib.rs` with `#[cfg(feature = "gui")]` (not just `pub use`) so non-GUI builds never compile GUI code.
  - Add `src/input/gui/app.rs` with a winit event loop that:
     - creates a window
     - initializes `pixels` with a surface size
     - handles resize + scale-factor-change events using physical size (`window.inner_size()` / `ScaleFactorChanged`)
     - drives a redraw loop
     - renders only on `RedrawRequested`, requesting redraw on input/resize to avoid a busy loop
     - requests an initial redraw after setup so the first frame appears without waiting for input

4. **Placeholder rendering**
   - Fill the `pixels` frame with a simple pattern or solid color each frame.
   - Keep it deterministic and cheap (no fractal render yet).
  - On resize, update both the pixels surface and buffer sizes (skip resize when the window is 0×0).
  - When the window is 0×0 (or otherwise invalid), skip rendering/presenting and keep the last valid frame.

5. **Minimal egui overlay**
   - Wire `egui` into the winit loop.
   - Use `pixels::render_with` (or equivalent hook) to render egui *after* the pixels pass, reusing the same `wgpu::Device`, `Queue`, and surface `TextureFormat`.
   - Render a small panel (e.g., label + slider) to prove integration.
   - Confirm the selected `pixels` version exposes the needed render hook and adjust integration to match its API.

6. **Library exports (if needed)**
   - Gate `pub use input::gui::run_gui;` behind `#[cfg(feature = "gui")]` in `src/lib.rs`.

7. **Build checks**
   - `cargo build` and `cargo test` succeed without `--features gui`.
   - `cargo run --features gui --bin gui` opens the window and shows the placeholder + egui panel.

## Out of Scope

This milestone focuses solely on the GUI skeleton. The following are explicitly **not** included:

- Fractal rendering (no calls to core fractal algorithms)
- `InteractiveController` or worker threads
- `FrameSink` port or presentation adapter
- `RenderRequest` / `RenderEvent` types
- Pan/zoom or other navigation controls
- Colour scheme or algorithm selection UI
- Any changes to `src/core/` or existing CLI functionality

These will be addressed in Milestones 2–5.

---

## Notes

- Keep all GUI types isolated under `input/gui` and avoid leaking them into `core`.
- Feature-gate GUI module declarations (not just exports) so non-GUI builds never compile GUI code.
- Use a single wgpu surface/device: render pixels first, then egui via a custom pass.
- Handle DPI/scale-factor changes using physical sizes when resizing the `pixels` surface.
- Avoid adding non-essential dependencies or refactors.

---

## Success Criteria

The milestone is complete when:

1. **Feature isolation verified**
   - `cargo build` succeeds without `--features gui`
   - `cargo test` succeeds without `--features gui`
   - Neither command pulls in `winit`, `pixels`, `egui`, or `wgpu` dependencies

2. **GUI binary runs**
   - `cargo run --features gui --bin gui` opens a window
   - Window displays a placeholder pattern (solid colour or gradient)
   - Resizing the window updates the rendered pattern without crashes

3. **Egui integration works**
   - A minimal egui panel is visible overlaid on the placeholder
   - The panel contains at least a label and an interactive widget (e.g., slider)
   - Interacting with the widget updates the UI (e.g., slider value changes)

4. **Resize/DPI handling is correct**
   - Resizing the window resizes both the pixels surface and buffer
   - Minimizing to 0×0 does not crash; rendering resumes when restored
   - High-DPI displays show correctly sized content

5. **Module structure matches spec**
   - GUI code lives under `src/input/gui/`
   - `src/bin/gui.rs` exists and is gated with `required-features = ["gui"]`
   - `src/lib.rs` gates GUI module declarations with `#[cfg(feature = "gui")]`

---

## Reference

For the complete specification including data models, threading model, cancellation strategy, and later milestones, see [SCREEN_RENDERING_SPEC.md](./SCREEN_RENDERING_SPEC.md).
