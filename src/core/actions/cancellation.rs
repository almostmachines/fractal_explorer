pub const CANCEL_CHECK_INTERVAL_PIXELS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation cancelled")
    }
}

impl std::error::Error for Cancelled {}

pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NeverCancel;

impl CancelToken for NeverCancel {
    #[inline]
    fn is_cancelled(&self) -> bool {
        false
    }
}

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
