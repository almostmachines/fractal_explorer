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
}
