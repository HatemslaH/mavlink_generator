//! Cooperative cancellation for MAVLink protocol waits.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;

/// Thrown when a MAVLink wait or long-running protocol operation is cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavlinkCancelledError {
    pub message: String,
}

impl MavlinkCancelledError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MavlinkCancelledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MavlinkCancelledError: {}", self.message)
    }
}

impl std::error::Error for MavlinkCancelledError {}

/// Cooperative cancellation token for [`super::mavlink_session::MavlinkSession`] waits.
#[derive(Clone)]
pub struct MavlinkCancellationToken {
    inner: Arc<Inner>,
}

struct Inner {
    cancelled: AtomicBool,
    notify: broadcast::Sender<()>,
}

impl MavlinkCancellationToken {
    pub fn new() -> Self {
        let (notify, _) = broadcast::channel(1);
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                notify,
            }),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        if self.inner.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.inner.notify.send(());
    }

    pub fn throw_if_cancelled(&self) -> Result<(), MavlinkCancelledError> {
        if self.is_cancelled() {
            Err(MavlinkCancelledError::new("Operation cancelled"))
        } else {
            Ok(())
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.inner.notify.subscribe()
    }
}

impl Default for MavlinkCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
