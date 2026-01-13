//! Output port for the interactive controller.
//!
//! Defines how render results are communicated to the presentation layer.
//! This is the output port in ports & adapters terminology, decoupling
//! the controller from any specific presentation implementation.

use std::time::Duration;

use crate::core::data::pixel_buffer::PixelBuffer;
use crate::core::data::pixel_rect::PixelRect;

/// A rendered frame ready for display.
#[derive(Debug)]
pub struct FrameMessage {
    /// Monotonic generation identifier for the request that produced this frame.
    pub generation: u64,
    /// Pixel-space bounds of the rendered frame.
    pub pixel_rect: PixelRect,
    /// RGB pixel data (3 bytes per pixel, row-major order).
    pub pixel_buffer: PixelBuffer,
    /// Time taken to render this frame.
    pub render_duration: Duration,
}

/// Information about a render error.
#[derive(Debug)]
pub struct RenderErrorMessage {
    /// Monotonic generation identifier for the request that failed.
    pub generation: u64,
    /// Human-readable error description.
    pub message: String,
}

/// Events emitted by the render pipeline.
#[derive(Debug)]
pub enum RenderEvent {
    /// A new frame is available for display.
    Frame(FrameMessage),
    /// An error occurred during rendering.
    Error(RenderErrorMessage),
}

/// Output port for receiving render events.
///
/// Implementations of this trait receive render results from the controller.
/// The trait bounds ensure thread safety:
/// - `Send`: The sink can be transferred to background render threads.
/// - `Sync`: The sink can be shared across threads for concurrent access.
///
/// Adapters implementing this trait might include:
/// - A GUI adapter that updates a texture
/// - A test adapter that collects frames for verification
/// - A network adapter that streams frames to clients
pub trait FrameSink: Send + Sync {
    /// Send a render event to the sink.
    fn submit(&self, event: RenderEvent);
}
