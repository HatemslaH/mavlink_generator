//! Transport-agnostic MAVLink byte stream.

use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::mpsc;

struct LinkRecvStream(mpsc::UnboundedReceiver<Vec<u8>>);

impl Stream for LinkRecvStream {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

/// Transport abstraction. Protocol code depends only on this trait.
#[async_trait]
pub trait MavlinkLink: Send + Sync {
    /// Send raw MAVLink frame bytes to the remote peer.
    async fn send(&self, data: &[u8]) -> Result<(), std::io::Error>;

    /// Incoming raw bytes from the remote peer.
    fn receive(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;

    /// Release link resources. Default implementation is a no-op.
    async fn close(&self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

/// In-memory link for tests and virtual examples.
///
/// Bytes sent by one endpoint are delivered to every other endpoint on the bus.
pub struct VirtualMavlinkBus {
    endpoints: Mutex<Vec<mpsc::UnboundedSender<Vec<u8>>>>,
}

impl VirtualMavlinkBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            endpoints: Mutex::new(Vec::new()),
        })
    }

    /// Create a new endpoint on this bus.
    pub fn create_endpoint(self: &Arc<Self>) -> Arc<dyn MavlinkLink> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.endpoints.lock().unwrap().push(tx.clone());
        Arc::new(VirtualMavlinkEndpoint {
            bus: Arc::downgrade(self),
            incoming_tx: tx,
            incoming_rx: Mutex::new(Some(rx)),
            closed: Mutex::new(false),
        })
    }

    fn deliver(&self, data: Vec<u8>, sender: &mpsc::UnboundedSender<Vec<u8>>) {
        let endpoints = self.endpoints.lock().unwrap();
        for endpoint in endpoints.iter() {
            if !std::ptr::eq(endpoint, sender) {
                let _ = endpoint.send(data.clone());
            }
        }
    }

    fn remove_endpoint(&self, tx: &mpsc::UnboundedSender<Vec<u8>>) {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.retain(|endpoint| !std::ptr::eq(endpoint, tx));
    }

    /// Close every endpoint on the bus.
    pub async fn close_all(self: &Arc<Self>) {
        let endpoints: Vec<_> = self.endpoints.lock().unwrap().drain(..).collect();
        drop(endpoints);
    }
}

struct VirtualMavlinkEndpoint {
    bus: Weak<VirtualMavlinkBus>,
    incoming_tx: mpsc::UnboundedSender<Vec<u8>>,
    incoming_rx: Mutex<Option<mpsc::UnboundedReceiver<Vec<u8>>>>,
    closed: Mutex<bool>,
}

#[async_trait]
impl MavlinkLink for VirtualMavlinkEndpoint {
    async fn send(&self, data: &[u8]) -> Result<(), std::io::Error> {
        if *self.closed.lock().unwrap() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "VirtualMavlinkEndpoint is closed",
            ));
        }
        if let Some(bus) = self.bus.upgrade() {
            bus.deliver(data.to_vec(), &self.incoming_tx);
        }
        Ok(())
    }

    fn receive(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>> {
        let rx = self
            .incoming_rx
            .lock()
            .unwrap()
            .take()
            .expect("receive stream already taken");
        Box::pin(LinkRecvStream(rx))
    }

    async fn close(&self) -> Result<(), std::io::Error> {
        let mut closed = self.closed.lock().unwrap();
        if *closed {
            return Ok(());
        }
        *closed = true;
        if let Some(bus) = self.bus.upgrade() {
            bus.remove_endpoint(&self.incoming_tx);
        }
        Ok(())
    }
}
