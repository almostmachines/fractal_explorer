//! Port definitions for the interactive controller.
//!
//! Contains trait definitions that define interfaces between the controller
//! and external systems (presentation layer, input sources, etc.).

mod frame_sink;

pub use frame_sink::{FrameSink, RenderErrorMessage, RenderEvent};
