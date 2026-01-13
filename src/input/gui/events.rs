/// Custom user events for the GUI event loop.
///
/// These events allow background threads (like the presenter) to wake
/// the main UI thread and trigger processing.
#[derive(Debug, Clone)]
pub enum GuiEvent {
    /// Signals that a new frame may be available from the presenter.
    ///
    /// Note: Receiving this event does NOT automatically trigger a redraw.
    /// The handler must explicitly call `window.request_redraw()` after
    /// checking if there's actually a new frame to display.
    Wake,
}
