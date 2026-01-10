# Milestone 1 Plan — GUI Skeleton (compile-ready)

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

## Notes
- Keep all GUI types isolated under `input/gui` and avoid leaking them into `core`.
- Feature-gate GUI module declarations (not just exports) so non-GUI builds never compile GUI code.
- Use a single wgpu surface/device: render pixels first, then egui via a custom pass.
- Handle DPI/scale-factor changes using physical sizes when resizing the `pixels` surface.
- No controller wiring or render jobs in this milestone.
- Avoid adding non-essential dependencies or refactors.
