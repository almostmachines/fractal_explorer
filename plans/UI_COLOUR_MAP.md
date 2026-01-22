# UI Colour Map Selection (egui ComboBox) - Final Plan

This is the final plan for adding GUI-only colour map selection for Mandelbrot via an
`egui::ComboBox`.

Scope: GUI-only colour map selection for Mandelbrot via an `egui::ComboBox`, triggering
a new render when the selection changes.

## Context (Repo Reality)

Relevant code today:

- GUI panel + render loop: `src/input/gui/app.rs`
- GUI state + request builder: `src/input/gui/ui_state.rs`
- Interactive render request: `src/controllers/interactive/data/fractal_config.rs`
- Colour map trait + enum kind: `src/core/fractals/mandelbrot/colour_map.rs`
- Mandelbrot colour maps: `src/core/fractals/mandelbrot/colour_maps/*`

Key facts that shape the implementation:

- `UiState` is constructed via `impl Default for UiState` (not `UiState::new()`).
- `UiState::build_render_request()` currently hardcodes `MandelbrotFireGradient`.
- `UiState::reset_view()` currently resets both region and `max_iterations`; this plan leaves
  that behavior unchanged and does not reset the colour map selection.
- Render submission is already driven by request comparison:
  `UiState::should_submit()` compares against `last_submitted_request`.
- `FractalConfig::PartialEq` compares colour maps by `colour_map.kind()`, so changing the
  kind automatically makes the request "different" and triggers a new generation.

## Goal

- Add a "Colour map" ComboBox to the debug panel.
- Populate it from *all* available `MandelbrotColourMapKind` variants.
- Store the selected kind in `UiState`.
- Switching selection triggers a new render without adding a "Render" button.
- Keep GUI code independent of concrete colour map types.

Non-goals:

- Persisting selection to disk.
- Palette preview UI.
- Adding new colour map implementations.

## Design

### 1) Store the selection as an enum in `UiState`

- Add `pub colour_map_kind: MandelbrotColourMapKind` to `UiState`.
- Default it in `impl Default for UiState` via `MandelbrotColourMapKind::default()`.
  - Implement `Default` for `MandelbrotColourMapKind` in core so the default is defined once
    and can later be reused for CLI defaults and persistence.
- Do not reset `colour_map_kind` in `UiState::reset_view()`.
  Rationale: users expect "Reset view" to affect region/zoom, not palette. If a full
  reset is desired later, add a separate "Reset all" action.

### 2) Add enum metadata for UI (all variants + canonical display labels)

`egui` needs:

- a stable list of options
- a user-facing label for each option

Add both on `MandelbrotColourMapKind` in `src/core/fractals/mandelbrot/colour_map.rs`:

```rust
impl MandelbrotColourMapKind {
    /// All supported Mandelbrot colour maps, in deterministic order (default first).
    ///
    /// Invariant: this list must contain every enum variant exactly once.
    pub const ALL: &'static [Self] = &[Self::FireGradient, Self::BlueWhiteGradient];

    /// Canonical human-readable label (single source of truth for UI/logging).
    ///
    /// Concrete implementations should implement `ColourMap::display_name()` by delegating to
    /// `self.kind().display_name()` to prevent string drift.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::FireGradient => "Fire gradient",
            Self::BlueWhiteGradient => "Blue-white gradient",
        }
    }
}

impl Default for MandelbrotColourMapKind {
    fn default() -> Self {
        Self::FireGradient
    }
}

impl std::fmt::Display for MandelbrotColourMapKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).display_name())
    }
}
```

Notes:

- Keep these strings identical to the current names, but do not duplicate them in concrete
  colour maps. Update each `ColourMap::display_name()` implementation to delegate to the
  enum, so the enum remains the single source of truth.
- The `Display` impl is a convenience for logging/debugging. Do not treat it as a stable
  persistence/serialization format (add a separate stable key if/when persistence is added).
- Use a `&'static [Self]` slice to avoid a manual `N` constant.
- Avoid index-based selection (`show_index`) unless you commit to stable repr/discriminants.

### 3) Centralize construction with a factory function

Add a Mandelbrot-specific factory function in `src/core/fractals/mandelbrot/colour_maps/mod.rs`:

```rust
use self::blue_white_gradient::MandelbrotBlueWhiteGradient;
use self::fire_gradient::MandelbrotFireGradient;
use super::colour_map::{MandelbrotColourMap, MandelbrotColourMapKind};

#[must_use]
pub fn mandelbrot_colour_map_factory(
    kind: MandelbrotColourMapKind,
    max_iterations: u32,
) -> Box<dyn MandelbrotColourMap> {
    match kind {
        MandelbrotColourMapKind::FireGradient => {
            Box::new(MandelbrotFireGradient::new(max_iterations))
        }
        MandelbrotColourMapKind::BlueWhiteGradient => {
            Box::new(MandelbrotBlueWhiteGradient::new(max_iterations))
        }
    }
}
```

Rationale:

- One wiring point for the kind -> concrete type mapping.
- GUI imports only the enum + factory.
- Adding a new map later becomes a simple checklist:
  add enum variant + update `display_name()` + add to `ALL` + add one factory match arm.

Export/import note (the "Plan 2" pitfall to avoid):

- Ensure `mandelbrot_colour_map_factory` is `pub` and reachable from GUI module paths.
  Prefer using direct module paths (consistent with current repo style) rather than adding
  new `pub use` re-exports unless you hit an actual import ergonomics problem.

### 4) Build the render request from the selected kind

Update `UiState::build_render_request()` in `src/input/gui/ui_state.rs`:

- Replace the hardcoded `MandelbrotFireGradient::new(...)` with:
  `mandelbrot_colour_map_factory(self.colour_map_kind, self.max_iterations)`.

This ensures:

- changing only `colour_map_kind` changes the resulting `FractalConfig`
- `UiState::should_submit()` sees a difference and submits a new generation

### 5) Add an egui ComboBox in the debug panel

In `src/input/gui/app.rs`, add a row near "Max iterations":

```rust
use crate::core::fractals::mandelbrot::colour_map::MandelbrotColourMapKind;

ui.horizontal(|ui| {
    ui.label("Colour map:");
    egui::ComboBox::from_id_salt("mandelbrot_colour_map")
        .selected_text(self.ui_state.colour_map_kind.display_name())
        .show_ui(ui, |ui| {
            for &kind in MandelbrotColourMapKind::ALL {
                ui.selectable_value(
                    &mut self.ui_state.colour_map_kind,
                    kind,
                    kind.display_name(),
                );
            }
        });
});
```

Notes:

- Use `selectable_value` (minimal state plumbing; matches current egui integration).
- Use a stable widget ID (`from_id_salt("mandelbrot_colour_map")`).
- No explicit repaint logic should be needed; selecting in egui generates input events
  and the request comparison triggers render submission.
- If a concrete repaint issue is observed, the fallback is `ctx.request_repaint()` when
  the selection changes (but do not change the event loop model preemptively).

## Testing

### Unit tests

1) Factory round-trip + labels (and "all variants covered")

- File: `src/core/fractals/mandelbrot/colour_maps/mod.rs`
- Assert `MandelbrotColourMapKind::ALL.first() == Some(&MandelbrotColourMapKind::default())`
  to keep the "default first" UI ordering invariant honest.
- For each `kind` in `MandelbrotColourMapKind::ALL`, build via the factory and assert
  `map.kind() == kind`.
- Also assert `map.display_name() == kind.display_name()`.
- Guardrail: assert all `display_name()` strings are unique to prevent ambiguous UI options.
- Note: Rust stable does not provide a stable `variant_count` API. Rely on code review + the
  documented `ALL` invariant when adding new `MandelbrotColourMapKind` variants.

2) `UiState` request changes when only kind changes

- File: `src/input/gui/ui_state.rs`
- Build two requests (or update `last_submitted_request`) with the same region/iterations
  but different `colour_map_kind`, and assert `should_submit()` flips to true.

Repository note:

- `src/input/gui/ui_state.rs` currently has stale commented tests; replace/remove those
  rather than adding more commented-out code.

### Manual GUI verification

Run:

```bash
cargo run --features gui --bin gui
```

Checklist:

- ComboBox appears in the debug panel.
- Options match the canonical labels.
- Switching selection submits a new generation and visibly changes the palette.

Optional screenshot-based check (Wayland):

```bash
cargo run --bin gui --features gui &
GUI_PID=$!
sleep 2
grim /tmp/colour_map_test.png
kill $GUI_PID
```

## File Change Summary

| File | Change |
|------|--------|
| `src/core/fractals/mandelbrot/colour_map.rs` | Add `MandelbrotColourMapKind::ALL`, `display_name()`, and `Default`/`Display` impls |
| `src/core/fractals/mandelbrot/colour_maps/mod.rs` | Add `mandelbrot_colour_map_factory()` factory + tests |
| `src/core/fractals/mandelbrot/colour_maps/fire_gradient.rs` | Delegate `display_name()` to `self.kind().display_name()` |
| `src/core/fractals/mandelbrot/colour_maps/blue_white_gradient.rs` | Delegate `display_name()` to `self.kind().display_name()` |
| `src/input/gui/ui_state.rs` | Add `colour_map_kind` to `UiState`, default it via `MandelbrotColourMapKind::default()`, use factory in `build_render_request()`, add tests |
| `src/input/gui/app.rs` | Add "Colour map" ComboBox using `selectable_value` |

## Acceptance Criteria

- GUI shows a "Colour map" ComboBox.
- ComboBox options come from `MandelbrotColourMapKind::ALL`, and labels come from
  `MandelbrotColourMapKind::display_name()` (matching current colour map names).
- Default selection comes from `MandelbrotColourMapKind::default()`.
- Clicking "Reset view" does not reset the colour map selection.
- Selection is stored in `UiState` and used to build subsequent `FractalConfig` values.
- Changing selection triggers a new render (new generation) without manual render controls.
- `cargo test` passes.

## Future Extensions (Optional Follow-ups)

- Palette preview strip next to the ComboBox.
- Persist UI state (including `colour_map_kind`) between runs.
- Apply the same pattern to other fractals if/when added.
