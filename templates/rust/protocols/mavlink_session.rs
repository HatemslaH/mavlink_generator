//! Framing, sequencing, and message dispatch over a [`super::mavlink_link::MavlinkLink`].

use std::any::Any;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::future::ready;
use futures::Stream;
use futures::StreamExt;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{self, Instant};
use tokio_stream::wrappers::BroadcastStream;

use crate::mavlink_dialect::MavlinkDialect;
use crate::mavlink_frame::MavlinkFrame;
use crate::mavlink_message::MavlinkMessage;
use crate::mavlink_parser::MavlinkParser;
use crate::mavlink_version::MavlinkVersion;

use super::mavlink_cancellation::{MavlinkCancellationToken, MavlinkCancelledError};
use super::mavlink_link::MavlinkLink;

/// Thrown when an expected MAVLink message is not received in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavlinkTimeoutError {
    pub message: String,
    pub timeout: Duration,
}

impl MavlinkTimeoutError {
    pub fn new(message: impl Into<String>, timeout: Duration) -> Self {
        Self {
            message: message.into(),
            timeout,
        }
    }
}

impl std::fmt::Display for MavlinkTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MavlinkTimeoutError: {} (timeout: {:?})",
            self.message, self.timeout
        )
    }
}

impl std::error::Error for MavlinkTimeoutError {}

/// Handle returned by [`MavlinkSession::listen_message`].
pub struct MavlinkMessageSubscription {
    active: Arc<AtomicBool>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl MavlinkMessageSubscription {
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn cancel(&mut self) {
        if !self.active.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
    }
}

struct PendingFrameWait {
    predicate: Box<dyn Fn(&MavlinkFrame) -> bool + Send>,
    respond: oneshot::Sender<Result<MavlinkFrame, SessionWaitError>>,
    deadline: Instant,
    timeout: Duration,
    cancel: Option<MavlinkCancellationToken>,
}

#[derive(Debug)]
pub enum SessionWaitError {
    Timeout(MavlinkTimeoutError),
    Cancelled(MavlinkCancelledError),
    Closed,
}

impl std::fmt::Display for SessionWaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(err) => write!(f, "{err}"),
            Self::Cancelled(err) => write!(f, "{err}"),
            Self::Closed => write!(f, "MavlinkSession is closed"),
        }
    }
}

impl std::error::Error for SessionWaitError {}

struct DialectArc(Arc<dyn MavlinkDialect + Send + Sync>);

impl MavlinkDialect for DialectArc {
    fn version(&self) -> u8 {
        self.0.version()
    }

    fn parse(&self, message_id: u32, data: &[u8]) -> Option<Box<dyn MavlinkMessage>> {
        self.0.parse(message_id, data)
    }

    fn crc_extra(&self, message_id: u32) -> i32 {
        self.0.crc_extra(message_id)
    }
}

struct SessionInner {
    dialect: Arc<dyn MavlinkDialect + Send + Sync>,
    system_id: u8,
    component_id: u8,
    version: MavlinkVersion,
    sequence: Mutex<u8>,
    frames_tx: broadcast::Sender<Arc<MavlinkFrame>>,
    pending_waits: Mutex<Vec<PendingFrameWait>>,
    recent_frames: Mutex<VecDeque<Arc<MavlinkFrame>>>,
    closed: AtomicBool,
}

/// Framing, sequencing, and message dispatch over a [`MavlinkLink`].
pub struct MavlinkSession {
    inner: Arc<SessionInner>,
    _reader: JoinHandle<()>,
    _link: Arc<dyn MavlinkLink>,
}

impl MavlinkSession {
    const RECENT_FRAME_CAPACITY: usize = 2048;

    pub fn new(
        dialect: Arc<dyn MavlinkDialect + Send + Sync>,
        link: Arc<dyn MavlinkLink>,
        system_id: u8,
        component_id: u8,
        version: MavlinkVersion,
    ) -> Self {
        let (frames_tx, _) = broadcast::channel(4096);
        let inner = Arc::new(SessionInner {
            dialect: Arc::clone(&dialect),
            system_id,
            component_id,
            version,
            sequence: Mutex::new(0),
            frames_tx,
            pending_waits: Mutex::new(Vec::new()),
            recent_frames: Mutex::new(VecDeque::new()),
            closed: AtomicBool::new(false),
        });

        let reader_inner = Arc::clone(&inner);
        let reader_link = Arc::clone(&link);
        let reader = tokio::spawn(async move {
            let mut parser =
                MavlinkParser::new(Box::new(DialectArc(Arc::clone(&reader_inner.dialect))));
            let mut receive = reader_link.receive();
            let mut parsed_cursor = 0usize;
            let mut ticker = time::interval(Duration::from_millis(20));

            loop {
                tokio::select! {
                    chunk = receive.next() => {
                        match chunk {
                            Some(data) => {
                                parser.parse(&data);
                                let frames = parser.frames();
                                for frame in &frames[parsed_cursor..] {
                                    if let Some(shared) = clone_frame(reader_inner.dialect.as_ref(), frame) {
                                        dispatch_frame(&reader_inner, shared);
                                    }
                                }
                                parsed_cursor = frames.len();
                            }
                            None => break,
                        }
                    }
                    _ = ticker.tick() => {
                        poll_timeouts(&reader_inner);
                    }
                }

                if reader_inner.closed.load(Ordering::SeqCst) {
                    break;
                }
            }
        });

        Self {
            inner,
            _reader: reader,
            _link: link,
        }
    }

    pub fn dialect(&self) -> &dyn MavlinkDialect {
        self.inner.dialect.as_ref()
    }

    pub fn system_id(&self) -> u8 {
        self.inner.system_id
    }

    pub fn component_id(&self) -> u8 {
        self.inner.component_id
    }

    pub fn version(&self) -> MavlinkVersion {
        self.inner.version
    }

    /// All frames parsed from the link (before filtering).
    pub fn frames(&self) -> impl Stream<Item = Arc<MavlinkFrame>> {
        BroadcastStream::new(self.inner.frames_tx.subscribe()).filter_map(
            |result: Result<Arc<MavlinkFrame>, tokio_stream::wrappers::errors::BroadcastStreamRecvError>| {
                ready(result.ok())
            },
        )
    }

    /// Typed message stream filtered by `from_system_id` / `from_component_id`.
    pub fn on_message<T>(
        &self,
        from_system_id: Option<u8>,
        from_component_id: Option<u8>,
    ) -> impl Stream<Item = Arc<T>>
    where
        T: MavlinkMessage + Clone + 'static,
    {
        self.frames()
            .filter_map(move |frame| {
                let matches_system = from_system_id.is_none_or(|id| frame.system_id == id);
                let matches_component =
                    from_component_id.is_none_or(|id| frame.component_id == id);
                if !matches_system || !matches_component {
                    return ready(None);
                }
                ready(
                    (frame.message.as_ref() as &dyn Any)
                        .downcast_ref::<T>()
                        .map(|message| Arc::new(message.clone())),
                )
            })
    }

    /// Message stream filtered by MAVLink message id.
    pub fn subscribe_message_id(
        &self,
        message_id: u32,
        from_system_id: Option<u8>,
        from_component_id: Option<u8>,
    ) -> impl Stream<Item = Box<dyn MavlinkMessage>> {
        let dialect = Arc::clone(&self.inner.dialect);
        self.frames().filter_map(move |frame| {
            if frame.message.mavlink_message_id() != message_id {
                return ready(None);
            }
            let matches_system = from_system_id.is_none_or(|id| frame.system_id == id);
            let matches_component = from_component_id.is_none_or(|id| frame.component_id == id);
            if matches_system && matches_component {
                ready(clone_message(dialect.as_ref(), frame.message.as_ref()))
            } else {
                ready(None)
            }
        })
    }

    /// Register a callback for messages of type `T`.
    pub fn listen_message<T, F>(
        &self,
        mut on_data: F,
        from_system_id: Option<u8>,
        from_component_id: Option<u8>,
    ) -> MavlinkMessageSubscription
    where
        T: MavlinkMessage + Clone + 'static,
        F: FnMut(Arc<T>, Arc<MavlinkFrame>) + Send + 'static,
    {
        let active = Arc::new(AtomicBool::new(true));
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        let mut frames = self.frames();

        let active_task = Arc::clone(&active);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    frame = frames.next() => {
                        if !active_task.load(Ordering::SeqCst) {
                            break;
                        }
                        let Some(frame) = frame else { break };
                        let matches_system = from_system_id.is_none_or(|id| frame.system_id == id);
                        let matches_component = from_component_id.is_none_or(|id| frame.component_id == id);
                        if !matches_system || !matches_component {
                            continue;
                        }
                        if let Some(message) = (frame.message.as_ref() as &dyn Any).downcast_ref::<T>() {
                            on_data(Arc::new(message.clone()), frame);
                        }
                    }
                    _ = &mut cancel_rx => break,
                }
            }
        });

        MavlinkMessageSubscription {
            active,
            cancel_tx: Some(cancel_tx),
        }
    }

    /// Send a typed MAVLink message as a framed packet.
    pub async fn send(&self, message: Box<dyn MavlinkMessage>) -> Result<(), SessionWaitError> {
        if self.inner.closed.load(Ordering::SeqCst) {
            return Err(SessionWaitError::Closed);
        }

        let wire = {
            let mut sequence = self.inner.sequence.lock().unwrap();
            let frame = match self.inner.version {
                MavlinkVersion::V1 => MavlinkFrame::v1(
                    *sequence,
                    self.inner.system_id,
                    self.inner.component_id,
                    message,
                ),
                MavlinkVersion::V2 => MavlinkFrame::v2(
                    *sequence,
                    self.inner.system_id,
                    self.inner.component_id,
                    message,
                ),
            };
            *sequence = sequence.wrapping_add(1);
            frame.serialize()
        };

        self._link
            .send(&wire)
            .await
            .map_err(|_| SessionWaitError::Closed)
    }

    /// Wait for the first frame matching `predicate`.
    pub async fn wait_for_frame(
        &self,
        predicate: impl Fn(&MavlinkFrame) -> bool + Send + 'static,
        timeout: Duration,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<MavlinkFrame, SessionWaitError> {
        if let Some(token) = cancel {
            token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
        }
        if self.inner.closed.load(Ordering::SeqCst) {
            return Err(SessionWaitError::Closed);
        }

        {
            let mut recent = self.inner.recent_frames.lock().unwrap();
            if let Some(index) = recent.iter().position(|frame| predicate(frame)) {
                let frame = recent.remove(index).unwrap();
                return clone_frame_owned(self.inner.dialect.as_ref(), frame.as_ref())
                    .ok_or(SessionWaitError::Closed);
            }
        }

        let (tx, rx) = oneshot::channel();
        let wait = PendingFrameWait {
            predicate: Box::new(predicate),
            respond: tx,
            deadline: Instant::now() + timeout,
            timeout,
            cancel: cancel.cloned(),
        };
        self.inner.pending_waits.lock().unwrap().push(wait);

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(SessionWaitError::Closed),
        }
    }

    /// Wait for the first message matching `predicate`.
    pub async fn wait_for_message(
        &self,
        predicate: impl Fn(&dyn MavlinkMessage) -> bool + Send + 'static,
        from_system_id: Option<u8>,
        from_component_id: Option<u8>,
        timeout: Duration,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<Box<dyn MavlinkMessage>, SessionWaitError> {
        let frame = self
            .wait_for_frame(
                move |frame| {
                    let matches_system = from_system_id.is_none_or(|id| frame.system_id == id);
                    let matches_component =
                        from_component_id.is_none_or(|id| frame.component_id == id);
                    matches_system && matches_component && predicate(frame.message.as_ref())
                },
                timeout,
                cancel,
            )
            .await?;
        Ok(frame.message)
    }

    /// Wait for the first message of type `T`.
    pub async fn wait_for_message_type<T>(
        &self,
        from_system_id: Option<u8>,
        from_component_id: Option<u8>,
        timeout: Duration,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<T, SessionWaitError>
    where
        T: MavlinkMessage + Clone + 'static,
    {
        let message = self
            .wait_for_message(
                |message| (message as &dyn Any).is::<T>(),
                from_system_id,
                from_component_id,
                timeout,
                cancel,
            )
            .await?;
        (message.as_ref() as &dyn Any)
            .downcast_ref::<T>()
            .cloned()
            .ok_or(SessionWaitError::Closed)
    }

    pub async fn close(&self) -> Result<(), SessionWaitError> {
        if self.inner.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let waits: Vec<_> = self.inner.pending_waits.lock().unwrap().drain(..).collect();
        for wait in waits {
            let _ = wait.respond.send(Err(SessionWaitError::Closed));
        }

        let _ = self._link.close().await;
        Ok(())
    }
}

fn clone_frame_owned(dialect: &dyn MavlinkDialect, frame: &MavlinkFrame) -> Option<MavlinkFrame> {
    let payload = frame.message.serialize();
    let message = dialect.parse(frame.message.mavlink_message_id(), &payload)?;
    Some(MavlinkFrame {
        version: frame.version,
        sequence: frame.sequence,
        system_id: frame.system_id,
        component_id: frame.component_id,
        message,
    })
}

fn clone_message(
    dialect: &dyn MavlinkDialect,
    message: &dyn MavlinkMessage,
) -> Option<Box<dyn MavlinkMessage>> {
    dialect.parse(message.mavlink_message_id(), &message.serialize())
}

fn clone_frame(dialect: &dyn MavlinkDialect, frame: &MavlinkFrame) -> Option<Arc<MavlinkFrame>> {
    clone_frame_owned(dialect, frame).map(Arc::new)
}

fn dispatch_frame(inner: &SessionInner, frame: Arc<MavlinkFrame>) {
    if inner.closed.load(Ordering::SeqCst) {
        return;
    }

    {
        let mut recent = inner.recent_frames.lock().unwrap();
        recent.push_back(Arc::clone(&frame));
        while recent.len() > MavlinkSession::RECENT_FRAME_CAPACITY {
            recent.pop_front();
        }
    }

    let mut waits = inner.pending_waits.lock().unwrap();
    if let Some(index) = waits.iter().position(|wait| (wait.predicate)(&frame)) {
        let wait = waits.remove(index);
        let result = clone_frame_owned(inner.dialect.as_ref(), frame.as_ref())
            .map(Ok)
            .unwrap_or(Err(SessionWaitError::Closed));
        let _ = wait.respond.send(result);
    }
    drop(waits);

    let _ = inner.frames_tx.send(Arc::clone(&frame));
}

fn poll_timeouts(inner: &SessionInner) {
    let now = Instant::now();
    let mut waits = inner.pending_waits.lock().unwrap();
    let mut index = 0;
    while index < waits.len() {
        let timed_out = waits[index].deadline <= now;
        let cancelled = waits[index]
            .cancel
            .as_ref()
            .is_some_and(|token| token.is_cancelled());
        if timed_out || cancelled {
            let wait = waits.remove(index);
            let result = if cancelled {
                Err(SessionWaitError::Cancelled(MavlinkCancelledError::new(
                    "Operation cancelled",
                )))
            } else {
                Err(SessionWaitError::Timeout(MavlinkTimeoutError::new(
                    "Timed out waiting for frame",
                    wait.timeout,
                )))
            };
            let _ = wait.respond.send(result);
        } else {
            index += 1;
        }
    }
}
