//! GUI input adapter for interactive fractal exploration.
//!
//! This module provides a windowed interface using winit for window management,
//! pixels for framebuffer rendering, and egui for UI controls.

mod app;
mod events;
mod ui_state;

pub use app::run_gui;
pub use events::GuiEvent;
pub use ui_state::UiState;
