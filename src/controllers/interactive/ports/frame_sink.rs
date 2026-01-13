//! Output port for the interactive controller.
//!
//! Defines how render results are communicated to the presentation layer.
//! This is the output port in ports & adapters terminology, decoupling
//! the controller from any specific presentation implementation.

/// A rendered frame ready for display.
#[derive(Debug, Clone)]
pub struct FrameMessage {
    /// RGB pixel data (3 bytes per pixel, row-major order).
    pub pixel_data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Time taken to render this frame in milliseconds.
    pub render_time_ms: u64,
}

/// Information about a render error.
#[derive(Debug, Clone)]
pub struct RenderErrorMessage {
    /// Human-readable error description.
    pub message: String,
    /// Whether the controller can continue operating after this error.
    pub recoverable: bool,
}

/// Events emitted by the render pipeline.
#[derive(Debug, Clone)]
pub enum RenderEvent {
    /// A new frame is available for display.
    FrameReady(FrameMessage),
    /// An error occurred during rendering.
    RenderError(RenderErrorMessage),
    /// Rendering has started (useful for UI feedback like spinners).
    RenderStarted,
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
    fn send_event(&self, event: RenderEvent);
}
