//! HEARTBEAT monitoring and publishing.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::{Heartbeat, MavAutopilot, MavState, MavType};
use super::mavlink_cancellation::MavlinkCancellationToken;
use super::mavlink_session::{MavlinkSession, MavlinkTimeoutError, SessionWaitError};

/// MAVLink node identity (system + component).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MavlinkNode {
    pub system_id: u8,
    pub component_id: u8,
}

impl MavlinkNode {
    pub fn new(system_id: u8, component_id: u8) -> Self {
        Self {
            system_id,
            component_id,
        }
    }
}

impl std::fmt::Display for MavlinkNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MavlinkNode({}:{})", self.system_id, self.component_id)
    }
}

/// Last known heartbeat state for a remote node.
#[derive(Debug, Clone)]
pub struct TrackedHeartbeat {
    pub node: MavlinkNode,
    pub heartbeat: Heartbeat,
    pub received_at: Instant,
    pub online: bool,
}

impl TrackedHeartbeat {
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }
}

struct MonitorInner {
    session: Arc<MavlinkSession>,
    timeout: Duration,
    watch: Option<HashSet<MavlinkNode>>,
    watch_system_id: Option<u8>,
    states: Mutex<HashMap<MavlinkNode, TrackedHeartbeat>>,
    online: Mutex<HashMap<MavlinkNode, bool>>,
    heartbeat_tx: broadcast::Sender<TrackedHeartbeat>,
    connected_tx: broadcast::Sender<MavlinkNode>,
    disconnected_tx: broadcast::Sender<MavlinkNode>,
    running: Mutex<bool>,
    reader: Mutex<Option<JoinHandle<()>>>,
    watchdog: Mutex<Option<JoinHandle<()>>>,
}

/// Tracks remote HEARTBEAT messages and reports connect / disconnect events.
pub struct HeartbeatMonitor {
    inner: Arc<MonitorInner>,
}

impl HeartbeatMonitor {
    pub fn new(
        session: Arc<MavlinkSession>,
        timeout: Duration,
        watch: Option<HashSet<MavlinkNode>>,
        watch_system_id: Option<u8>,
    ) -> Self {
        let (heartbeat_tx, _) = broadcast::channel(64);
        let (connected_tx, _) = broadcast::channel(64);
        let (disconnected_tx, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(MonitorInner {
                session,
                timeout,
                watch,
                watch_system_id,
                states: Mutex::new(HashMap::new()),
                online: Mutex::new(HashMap::new()),
                heartbeat_tx,
                connected_tx,
                disconnected_tx,
                running: Mutex::new(false),
                reader: Mutex::new(None),
                watchdog: Mutex::new(None),
            }),
        }
    }

    pub fn on_heartbeat(&self) -> broadcast::Receiver<TrackedHeartbeat> {
        self.inner.heartbeat_tx.subscribe()
    }

    pub fn on_connected(&self) -> broadcast::Receiver<MavlinkNode> {
        self.inner.connected_tx.subscribe()
    }

    pub fn on_disconnected(&self) -> broadcast::Receiver<MavlinkNode> {
        self.inner.disconnected_tx.subscribe()
    }

    pub fn start(&self) {
        let mut running = self.inner.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;

        let inner = Arc::clone(&self.inner);
        let reader = tokio::spawn(async move {
            let mut frames = inner.session.frames();
            while let Some(frame) = frames.next().await {
                if !*inner.running.lock().unwrap() {
                    break;
                }
                let Some(heartbeat) = (frame.message.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<Heartbeat>()
                    .cloned()
                else {
                    continue;
                };
                let node = MavlinkNode::new(frame.system_id, frame.component_id);
                if !should_watch(&inner, node) {
                    continue;
                }

                let was_online = *inner.online.lock().unwrap().get(&node).unwrap_or(&false);
                let tracked = TrackedHeartbeat {
                    node,
                    heartbeat,
                    received_at: Instant::now(),
                    online: true,
                };
                inner.states.lock().unwrap().insert(node, tracked.clone());
                inner.online.lock().unwrap().insert(node, true);
                let _ = inner.heartbeat_tx.send(tracked);
                if !was_online {
                    let _ = inner.connected_tx.send(node);
                }
            }
        });
        *self.inner.reader.lock().unwrap() = Some(reader);

        let inner = Arc::clone(&self.inner);
        let period = self.inner.timeout.div_f32(3.0).max(Duration::from_millis(100));
        let watchdog = tokio::spawn(async move {
            let mut ticker = time::interval(period);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if !*inner.running.lock().unwrap() {
                    break;
                }
                check_timeouts(&inner);
            }
        });
        *self.inner.watchdog.lock().unwrap() = Some(watchdog);
    }

    pub async fn stop(&self) {
        let mut running = self.inner.running.lock().unwrap();
        if !*running {
            return;
        }
        *running = false;
        if let Some(handle) = self.inner.reader.lock().unwrap().take() {
            handle.abort();
        }
        if let Some(handle) = self.inner.watchdog.lock().unwrap().take() {
            handle.abort();
        }
    }

    pub fn state_for(&self, node: MavlinkNode) -> Option<TrackedHeartbeat> {
        self.inner.states.lock().unwrap().get(&node).cloned()
    }

    pub fn state_for_ids(&self, system_id: u8, component_id: u8) -> Option<TrackedHeartbeat> {
        self.state_for(MavlinkNode::new(system_id, component_id))
    }

    pub fn is_online(&self, node: MavlinkNode) -> bool {
        *self.inner.online.lock().unwrap().get(&node).unwrap_or(&false)
    }

    pub fn is_online_ids(&self, system_id: u8, component_id: u8) -> bool {
        self.is_online(MavlinkNode::new(system_id, component_id))
    }

    pub fn online_nodes(&self) -> Vec<MavlinkNode> {
        self.inner
            .online
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(node, online)| online.then_some(*node))
            .collect()
    }

    pub async fn wait_for_vehicle(
        &self,
        exclude_system_ids: Option<HashSet<u8>>,
        timeout: Duration,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<MavlinkNode, SessionWaitError> {
        if let Some(token) = cancel {
            token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
        }

        for node in self.online_nodes() {
            if exclude_system_ids
                .as_ref()
                .is_none_or(|ids| !ids.contains(&node.system_id))
            {
                return Ok(node);
            }
        }

        let (tx, rx) = oneshot::channel();
        let mut connected = self.on_connected();
        let waiter = tokio::spawn(async move {
            while let Ok(node) = connected.recv().await {
                if exclude_system_ids
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&node.system_id))
                {
                    continue;
                }
                let _ = tx.send(node);
                break;
            }
        });

        let result = time::timeout(timeout, rx).await;
        waiter.abort();

        match result {
            Ok(Ok(node)) => Ok(node),
            Ok(Err(_)) => Err(SessionWaitError::Closed),
            Err(_) => Err(SessionWaitError::Timeout(MavlinkTimeoutError::new(
                "Timed out waiting for vehicle heartbeat",
                timeout,
            ))),
        }
    }
}

fn should_watch(inner: &MonitorInner, node: MavlinkNode) -> bool {
    if let Some(watch) = &inner.watch {
        return watch.contains(&node);
    }
    if let Some(system_id) = inner.watch_system_id {
        return node.system_id == system_id;
    }
    true
}

fn check_timeouts(inner: &MonitorInner) {
    let now = Instant::now();
    let nodes: Vec<_> = inner.states.lock().unwrap().keys().copied().collect();
    for node in nodes {
        let Some(state) = inner.states.lock().unwrap().get(&node).cloned() else {
            continue;
        };
        let timed_out = now.duration_since(state.received_at) > inner.timeout;
        let was_online = *inner.online.lock().unwrap().get(&node).unwrap_or(&false);
        if timed_out && was_online {
            inner.online.lock().unwrap().insert(node, false);
            let _ = inner.disconnected_tx.send(node);
            let _ = inner.heartbeat_tx.send(TrackedHeartbeat {
                node,
                heartbeat: state.heartbeat,
                received_at: state.received_at,
                online: false,
            });
        }
    }
}

/// Periodically sends HEARTBEAT on a [`MavlinkSession`].
pub struct HeartbeatPublisher {
    session: Arc<MavlinkSession>,
    interval: Duration,
    heartbeat: Mutex<Heartbeat>,
    timer: Mutex<Option<JoinHandle<()>>>,
    running: Mutex<bool>,
}

impl HeartbeatPublisher {
    pub fn new(session: Arc<MavlinkSession>, heartbeat: Heartbeat, interval: Duration) -> Self {
        Self {
            session,
            interval,
            heartbeat: Mutex::new(heartbeat),
            timer: Mutex::new(None),
            running: Mutex::new(false),
        }
    }

    pub fn heartbeat(&self) -> Heartbeat {
        self.heartbeat.lock().unwrap().clone()
    }

    pub fn update_heartbeat(&self, heartbeat: Heartbeat) {
        *self.heartbeat.lock().unwrap() = heartbeat;
    }

    pub fn mutate_heartbeat(&self, transform: impl FnOnce(Heartbeat) -> Heartbeat) {
        let mut heartbeat = self.heartbeat.lock().unwrap();
        *heartbeat = transform(heartbeat.clone());
    }

    pub fn start(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        let session = Arc::clone(&self.session);
        let interval = self.interval;
        let heartbeat = Arc::new(Mutex::new(self.heartbeat.lock().unwrap().clone()));
        let running_flag = Arc::new(Mutex::new(true));
        let timer_running = Arc::clone(&running_flag);
        let timer = tokio::spawn(async move {
            let _ = send_once(&session, &heartbeat).await;
            let mut ticker = time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if !*timer_running.lock().unwrap() {
                    break;
                }
                let _ = send_once(&session, &heartbeat).await;
            }
        });
        *self.timer.lock().unwrap() = Some(timer);
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        if let Some(handle) = self.timer.lock().unwrap().take() {
            handle.abort();
        }
    }

    pub async fn send_once(&self) -> Result<(), SessionWaitError> {
        send_once(&self.session, &self.heartbeat).await
    }
}

async fn send_once(
    session: &MavlinkSession,
    heartbeat: &Mutex<Heartbeat>,
) -> Result<(), SessionWaitError> {
    let heartbeat = heartbeat.lock().unwrap().clone();
    session.send(Box::new(heartbeat)).await
}

/// Convenience factories for common HEARTBEAT payloads.
pub struct HeartbeatTemplates;

impl HeartbeatTemplates {
    pub fn gcs(mavlink_version: u8) -> Heartbeat {
        Heartbeat {
            custom_mode: 0,
            r#type: MavType::MAV_TYPE_GCS,
            autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: 0,
            system_status: MavState::MAV_STATE_ACTIVE,
            mavlink_version,
        }
    }

    pub fn autopilot(
        mavlink_version: u8,
        vehicle_type: MavType,
        autopilot: MavAutopilot,
        system_status: MavState,
        custom_mode: u32,
        base_mode: u8,
    ) -> Heartbeat {
        Heartbeat {
            custom_mode,
            r#type: vehicle_type,
            autopilot,
            base_mode,
            system_status,
            mavlink_version,
        }
    }

    pub fn autopilot_default(mavlink_version: u8) -> Heartbeat {
        Self::autopilot(
            mavlink_version,
            MavType::MAV_TYPE_QUADROTOR,
            MavAutopilot::MAV_AUTOPILOT_PX4,
            MavState::MAV_STATE_ACTIVE,
            0,
            0,
        )
    }

    pub fn onboard_api(mavlink_version: u8) -> Heartbeat {
        Heartbeat {
            custom_mode: 0,
            r#type: MavType::MAV_TYPE_ONBOARD_CONTROLLER,
            autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: 0,
            system_status: MavState::MAV_STATE_ACTIVE,
            mavlink_version,
        }
    }
}
