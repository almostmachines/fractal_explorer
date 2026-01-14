/// Cooperative cancellation primitives for long-running operations.
///
/// This module provides allocation-free cancellation tokens suitable for use
/// in tight loops. The design allows callers to periodically check if work
/// should be cancelled without incurring heap allocations.
///
/// # Cancellation Semantics
///
/// Cancellation is **expected control flow**, not an error condition. When an
/// operation returns [`Cancelled`], callers should:
///
/// - Silently discard the incomplete result
/// - **Not** propagate this as a `RenderEvent::Error`
/// - Allow the next render request to proceed normally
///
/// This distinction matters because cancellation typically occurs when a newer
/// render request supersedes an in-progress one (e.g., during rapid panning).
/// Users should not see error messages for this normal interaction pattern.
///
/// # Usage Pattern
///
/// Actions should check the cancellation token periodically using
/// [`CANCEL_CHECK_INTERVAL_PIXELS`] to balance responsiveness against overhead:
///
/// ```ignore
/// for (i, pixel) in pixels.iter().enumerate() {
///     if i % CANCEL_CHECK_INTERVAL_PIXELS == 0 && token.is_cancelled() {
///         return Err(Cancelled);
///     }
///     // ... process pixel ...
/// }
/// ```

/// How often (in pixels) to check for cancellation during iteration loops.
/// Balances responsiveness against check overhead.
pub const CANCEL_CHECK_INTERVAL_PIXELS: usize = 1024;

/// Marker type indicating an operation was cancelled.
///
/// Used as explicit control flow to distinguish cancellation from other
/// error conditions. This is a zero-sized type with no runtime overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Token that can be polled to check if an operation should be cancelled.
///
/// Implementations must be thread-safe (`Send + Sync`) to support parallel
/// execution. The `is_cancelled` method should be cheap to call repeatedly.
pub trait CancelToken: Send + Sync {
    /// Returns `true` if cancellation has been requested.
    fn is_cancelled(&self) -> bool;
}

/// A cancellation token that never signals cancellation.
///
/// Use this for call sites that don't support cancellation, avoiding the
/// need for `Option<&dyn CancelToken>` or similar patterns.
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverCancel;

impl CancelToken for NeverCancel {
    #[inline]
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Blanket implementation allowing closures to serve as cancellation tokens.
///
/// This enables flexible cancellation sources without requiring new types:
///
/// ```ignore
/// let flag = AtomicBool::new(false);
/// let token = || flag.load(Ordering::Relaxed);
/// // token can now be used as a CancelToken
/// ```
impl<F> CancelToken for F
where
    F: Fn() -> bool + Send + Sync,
{
    #[inline]
    fn is_cancelled(&self) -> bool {
        self()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn never_cancel_always_returns_false() {
        let token = NeverCancel;
        assert!(!token.is_cancelled());
        assert!(!token.is_cancelled());
    }

    #[test]
    fn closure_token_reflects_atomic_state() {
        let flag = AtomicBool::new(false);
        let token = || flag.load(Ordering::Relaxed);

        assert!(!token.is_cancelled());

        flag.store(true, Ordering::Relaxed);
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancelled_displays_message() {
        let cancelled = Cancelled;
        assert_eq!(format!("{}", cancelled), "operation cancelled");
    }

    #[test]
    fn cancelled_implements_error() {
        let cancelled: &dyn std::error::Error = &Cancelled;
        assert!(cancelled.to_string().contains("cancelled"));
    }

    #[test]
    fn cancel_check_interval_is_reasonable() {
        // Should be large enough to amortize check overhead
        assert!(CANCEL_CHECK_INTERVAL_PIXELS >= 256);
        // But not so large that cancellation is unresponsive
        assert!(CANCEL_CHECK_INTERVAL_PIXELS <= 8192);
    }

    #[test]
    fn never_cancel_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NeverCancel>();
    }

    #[test]
    fn closure_token_is_send_and_sync() {
        fn assert_cancel_token<T: CancelToken>() {}
        let flag = AtomicBool::new(false);
        // The closure captures a reference, so we test a standalone version
        let _token = || false;
        assert_cancel_token::<fn() -> bool>();
        // Also verify the captured version works
        let _ = flag.load(Ordering::Relaxed);
        assert_cancel_token::<fn() -> bool>();
    }
}
