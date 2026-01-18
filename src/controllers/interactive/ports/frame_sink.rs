//! Output port for the interactive controller.
//!
//! Defines how render results are communicated to the presentation layer.
//! This is the output port in ports & adapters terminology, decoupling
//! the controller from any specific presentation implementation.

use crate::controllers::interactive::data::frame_data::FrameData;

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
    Frame(FrameData),
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
