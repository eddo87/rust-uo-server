use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// A cross-platform shutdown signal that can be shared across threads.
///
/// Uses an `Arc<AtomicBool>` internally so cloning is cheap and all clones
/// observe the same shutdown state.
#[derive(Clone)]
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new `ShutdownSignal` in the non-shutdown state.
    pub fn new() -> Self {
        ShutdownSignal {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns `true` if a shutdown has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// Request a shutdown. All clones of this signal will observe the change.
    pub fn request_shutdown(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Block the calling thread until a shutdown is requested, polling every
    /// `poll_interval`.
    pub fn wait_for_shutdown(&self, poll_interval: Duration) {
        while !self.is_shutdown_requested() {
            thread::sleep(poll_interval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn new_signal_is_not_shutdown() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_shutdown_requested());
    }

    #[test]
    fn request_shutdown_sets_flag() {
        let signal = ShutdownSignal::new();
        signal.request_shutdown();
        assert!(signal.is_shutdown_requested());
    }

    #[test]
    fn clones_share_state() {
        let signal = ShutdownSignal::new();
        let signal_clone = signal.clone();

        assert!(!signal_clone.is_shutdown_requested());
        signal.request_shutdown();
        assert!(signal_clone.is_shutdown_requested());
    }

    #[test]
    fn clone_can_trigger_shutdown_for_original() {
        let signal = ShutdownSignal::new();
        let signal_clone = signal.clone();

        signal_clone.request_shutdown();
        assert!(signal.is_shutdown_requested());
    }

    #[test]
    fn wait_for_shutdown_returns_when_signalled() {
        let signal = ShutdownSignal::new();
        let signal_clone = signal.clone();

        let handle = thread::spawn(move || {
            let start = Instant::now();
            signal_clone.wait_for_shutdown(Duration::from_millis(10));
            start.elapsed()
        });

        // Give the spawned thread time to start waiting.
        thread::sleep(Duration::from_millis(50));
        signal.request_shutdown();

        let elapsed = handle.join().expect("thread should not panic");
        // The wait should have ended shortly after we signalled (within a
        // generous bound to avoid flaky CI).
        assert!(
            elapsed < Duration::from_secs(2),
            "wait_for_shutdown took too long: {:?}",
            elapsed,
        );
    }

    #[test]
    fn multiple_shutdown_requests_are_idempotent() {
        let signal = ShutdownSignal::new();
        signal.request_shutdown();
        signal.request_shutdown();
        assert!(signal.is_shutdown_requested());
    }

    #[test]
    fn many_clones_all_see_shutdown() {
        let signal = ShutdownSignal::new();
        let clones: Vec<_> = (0..100).map(|_| signal.clone()).collect();

        signal.request_shutdown();

        for (i, s) in clones.iter().enumerate() {
            assert!(
                s.is_shutdown_requested(),
                "clone {} did not see shutdown",
                i,
            );
        }
    }
}
