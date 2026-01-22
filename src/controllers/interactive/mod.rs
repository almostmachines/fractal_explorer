//! Interactive controller for real-time fractal rendering.
//!
//! This module provides the application layer for interactive fractal exploration,
//! managing render requests and dispatching results to the presentation layer.
//!
//! # Architecture
//!
//! The interactive controller follows the ports & adapters pattern:
//! - **Input**: `RenderRequest` structs describing what to render
//! - **Output**: `PresenterPort` trait for receiving rendered frames
//! - **Core**: Uses domain actions from `core/` for actual computation

mod controller;
pub mod ports;
pub mod data;
pub mod errors;
pub mod events;

pub use controller::InteractiveController;
