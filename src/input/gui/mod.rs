//! GUI input adapter for interactive fractal exploration.
//!
//! This module provides a windowed interface using winit for window management,
//! pixels for framebuffer rendering, and egui for UI controls.

mod app;

pub use app::run_gui;
