//! [`MavlinkLink`] implementation over a serial/COM port.

use std::io::{Read, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use mavlink::protocols::MavlinkLink;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

struct LinkRecvStream(mpsc::UnboundedReceiver<Vec<u8>>);

impl Stream for LinkRecvStream {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

/// [`MavlinkLink`] over a host serial port (USB virtual COM, SITL telemetry link, etc.).
pub struct SerialMavlinkLink {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    incoming_rx: Mutex<Option<mpsc::UnboundedReceiver<Vec<u8>>>>,
    closed: Arc<AtomicBool>,
    read_task: Mutex<Option<JoinHandle<()>>>,
}

impl SerialMavlinkLink {
    /// Open [port_name] at [baud_rate] (MAVLink SITL commonly uses 57600 or 115200).
    pub fn open(port_name: &str, baud_rate: u32) -> std::io::Result<Arc<Self>> {
        let mut port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(50))
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .open()?;
        let _ = port.write_data_terminal_ready(true);
        let _ = port.write_request_to_send(true);

        let port = Arc::new(Mutex::new(port));
        let (tx, rx) = mpsc::unbounded_channel();
        let closed = Arc::new(AtomicBool::new(false));

        let link = Arc::new(Self {
            port: Arc::clone(&port),
            incoming_rx: Mutex::new(Some(rx)),
            closed: Arc::clone(&closed),
            read_task: Mutex::new(None),
        });

        let read_port = Arc::clone(&port);
        let read_closed = Arc::clone(&closed);
        let read_task = tokio::task::spawn_blocking(move || {
            let mut buffer = [0u8; 4096];
            while !read_closed.load(Ordering::SeqCst) {
                let read_result = {
                    let mut guard = match read_port.lock() {
                        Ok(guard) => guard,
                        Err(_) => break,
                    };
                    guard.read(&mut buffer)
                };

                match read_result {
                    Ok(0) => continue,
                    Ok(count) => {
                        if tx.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(_) => break,
                }
            }
        });
        *link.read_task.lock().unwrap() = Some(read_task);

        Ok(link)
    }
}

#[async_trait]
impl MavlinkLink for SerialMavlinkLink {
    async fn send(&self, data: &[u8]) -> std::io::Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "SerialMavlinkLink is closed",
            ));
        }

        let port = Arc::clone(&self.port);
        let payload = data.to_vec();
        tokio::task::spawn_blocking(move || {
            let mut guard = port
                .lock()
                .map_err(|_| std::io::Error::other("serial port mutex poisoned"))?;
            guard.write_all(&payload)?;
            guard.flush()
        })
        .await
        .map_err(|_| std::io::Error::other("serial write task failed"))??;
        Ok(())
    }

    fn receive(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>> {
        let mut guard = self.incoming_rx.lock().unwrap();
        let rx = guard.take().unwrap_or_else(|| mpsc::unbounded_channel().1);
        Box::pin(LinkRecvStream(rx))
    }

    async fn close(&self) -> std::io::Result<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let task = self.read_task.lock().unwrap().take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }

        Ok(())
    }
}
