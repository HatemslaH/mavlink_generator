//! MAVLink mission protocol client and server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use tokio::task::JoinHandle;

use crate::{
    CommandAck, MavComponent, MavFrame, MavCmd, MavMissionResult, MavMissionType, MissionAck,
    MissionClearAll, MissionCount, MissionItem, MissionItemInt, MissionRequest, MissionRequestInt,
    MissionRequestList, MissionSetCurrent,
};

use super::command_protocol::CommandProtocol;
use super::mavlink_cancellation::MavlinkCancellationToken;
use super::mavlink_session::{MavlinkSession, SessionWaitError};

/// Helpers for building and converting mission plan items.
pub struct MissionItems;

impl MissionItems {
    pub fn waypoint(
        seq: u16,
        latitude: f64,
        longitude: f64,
        altitude: f32,
        target_system: u8,
        target_component: u8,
        command: MavCmd,
        frame: MavFrame,
        mission_type: MavMissionType,
        param1: f32,
        param2: f32,
        param3: f32,
        param4: f32,
        current: u8,
        autocontinue: u8,
    ) -> MissionItemInt {
        MissionItemInt {
            param1,
            param2,
            param3,
            param4,
            x: (latitude * 1e7).round() as i32,
            y: (longitude * 1e7).round() as i32,
            z: altitude,
            seq,
            command,
            target_system,
            target_component,
            frame,
            current,
            autocontinue,
            mission_type,
        }
    }

    pub fn to_legacy_item(item: &MissionItemInt) -> MissionItem {
        MissionItem {
            param1: item.param1,
            param2: item.param2,
            param3: item.param3,
            param4: item.param4,
            x: item.x as f32 / 1e7,
            y: item.y as f32 / 1e7,
            z: item.z,
            seq: item.seq,
            command: item.command,
            target_system: item.target_system,
            target_component: item.target_component,
            frame: item.frame,
            current: item.current,
            autocontinue: item.autocontinue,
            mission_type: item.mission_type,
        }
    }

    pub fn from_legacy_item(item: &MissionItem) -> MissionItemInt {
        MissionItemInt {
            param1: item.param1,
            param2: item.param2,
            param3: item.param3,
            param4: item.param4,
            x: (item.x as f64 * 1e7).round() as i32,
            y: (item.y as f64 * 1e7).round() as i32,
            z: item.z,
            seq: item.seq,
            command: item.command,
            target_system: item.target_system,
            target_component: item.target_component,
            frame: item.frame,
            current: item.current,
            autocontinue: item.autocontinue,
            mission_type: item.mission_type,
        }
    }

    pub fn with_sequential_seq(items: Vec<MissionItemInt>) -> Vec<MissionItemInt> {
        items
            .into_iter()
            .enumerate()
            .map(|(index, item)| MissionItemInt { seq: index as u16, ..item })
            .collect()
    }
}

pub type MissionUploadProgressCallback = dyn Fn(usize, usize, &MissionItemInt) + Send + Sync;
pub type MissionDownloadProgressCallback = dyn Fn(usize, u16, &MissionItemInt) + Send + Sync;

/// Result of [`MissionProtocol::set_current_with_command`].
#[derive(Debug, Clone)]
pub struct MissionSetCurrentResult {
    pub sequence: u16,
    pub command_ack: Option<CommandAck>,
}

/// GCS-side MAVLink mission protocol client.
pub struct MissionProtocol {
    session: Arc<MavlinkSession>,
    target_system: u8,
    target_component: u8,
    item_timeout: Duration,
    operation_timeout: Duration,
}

impl MissionProtocol {
    pub fn new(
        session: Arc<MavlinkSession>,
        target_system: u8,
        target_component: u8,
        item_timeout: Duration,
        operation_timeout: Duration,
    ) -> Self {
        Self {
            session,
            target_system,
            target_component,
            item_timeout,
            operation_timeout,
        }
    }

    pub async fn upload(
        &self,
        items: Vec<MissionItemInt>,
        mission_type: MavMissionType,
        on_progress: Option<&MissionUploadProgressCallback>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<MavMissionResult, SessionWaitError> {
        if let Some(token) = cancel {
            token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
        }
        let plan = MissionItems::with_sequential_seq(items);

        self.session
            .send(Box::new(MissionCount {
                count: plan.len() as u16,
                target_system: self.target_system,
                target_component: self.target_component,
                mission_type,
            }))
            .await?;

        for item in &plan {
            if let Some(token) = cancel {
                token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
            }

            let seq = item.seq;
            let request = self
                .session
                .wait_for_message(
                    move |message| is_item_request(message, seq, mission_type),
                    Some(self.target_system),
                    None,
                    self.item_timeout,
                    cancel,
                )
                .await?;

            if (request.as_ref() as &dyn std::any::Any)
                .downcast_ref::<MissionRequestInt>()
                .is_some()
            {
                self.session.send(Box::new(item.clone())).await?;
            } else if (request.as_ref() as &dyn std::any::Any)
                .downcast_ref::<MissionRequest>()
                .is_some()
            {
                self.session
                    .send(Box::new(MissionItems::to_legacy_item(item)))
                    .await?;
            }

            if let Some(callback) = on_progress {
                callback(item.seq as usize + 1, plan.len(), item);
            }
        }

        let ack = self
            .session
            .wait_for_message_type::<MissionAck>(
                Some(self.target_system),
                None,
                self.operation_timeout,
                cancel,
            )
            .await?;

        Ok(ack.r#type)
    }

    pub async fn download(
        &self,
        mission_type: MavMissionType,
        on_progress: Option<&MissionDownloadProgressCallback>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<Vec<MissionItemInt>, SessionWaitError> {
        if let Some(token) = cancel {
            token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
        }

        self.session
            .send(Box::new(MissionRequestList {
                target_system: self.target_system,
                target_component: self.target_component,
                mission_type,
            }))
            .await?;

        let count_message = self
            .session
            .wait_for_message_type::<MissionCount>(
                Some(self.target_system),
                None,
                self.operation_timeout,
                cancel,
            )
            .await?;

        let mut items = Vec::new();
        for seq in 0..count_message.count {
            if let Some(token) = cancel {
                token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
            }

            self.session
                .send(Box::new(MissionRequestInt {
                    seq,
                    target_system: self.target_system,
                    target_component: self.target_component,
                    mission_type,
                }))
                .await?;

            let item_message = self
                .session
                .wait_for_message(
                    move |message| match_message_item(message, seq, mission_type),
                    Some(self.target_system),
                    None,
                    self.item_timeout,
                    cancel,
                )
                .await?;

            let item = if let Some(item) =
                (item_message.as_ref() as &dyn std::any::Any).downcast_ref::<MissionItemInt>()
            {
                item.clone()
            } else if let Some(item) =
                (item_message.as_ref() as &dyn std::any::Any).downcast_ref::<MissionItem>()
            {
                MissionItems::from_legacy_item(item)
            } else {
                return Err(SessionWaitError::Closed);
            };

            items.push(item.clone());
            if let Some(callback) = on_progress {
                callback(items.len(), count_message.count, &item);
            }
        }

        self.session
            .send(Box::new(MissionAck {
                target_system: self.target_system,
                target_component: self.target_component,
                r#type: MavMissionResult::MAV_MISSION_ACCEPTED,
                mission_type,
            }))
            .await?;

        Ok(items)
    }

    pub async fn clear(
        &self,
        mission_type: MavMissionType,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<MavMissionResult, SessionWaitError> {
        self.session
            .send(Box::new(MissionClearAll {
                target_system: self.target_system,
                target_component: self.target_component,
                mission_type,
            }))
            .await?;

        let ack = self
            .session
            .wait_for_message_type::<MissionAck>(
                Some(self.target_system),
                None,
                self.operation_timeout,
                cancel,
            )
            .await?;

        Ok(ack.r#type)
    }

    pub async fn set_current(
        &self,
        seq: u16,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<(), SessionWaitError> {
        if let Some(token) = cancel {
            token.throw_if_cancelled().map_err(SessionWaitError::Cancelled)?;
        }
        self.session
            .send(Box::new(MissionSetCurrent {
                seq,
                target_system: self.target_system,
                target_component: self.target_component,
            }))
            .await
    }

    pub async fn set_current_with_command(
        &self,
        seq: u16,
        command: Option<&CommandProtocol>,
        also_send_command: bool,
        reset_mission: bool,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<MissionSetCurrentResult, SessionWaitError> {
        self.set_current(seq, cancel).await?;
        let command_ack = if also_send_command {
            if let Some(command) = command {
                Some(
                    command
                        .set_mission_current(seq, reset_mission, None, cancel)
                        .await?,
                )
            } else {
                None
            }
        } else {
            None
        };
        Ok(MissionSetCurrentResult {
            sequence: seq,
            command_ack,
        })
    }
}

fn is_item_request(message: &dyn crate::mavlink_message::MavlinkMessage, seq: u16, mission_type: MavMissionType) -> bool {
    if let Some(request) = (message as &dyn std::any::Any).downcast_ref::<MissionRequestInt>() {
        return request.seq == seq && request.mission_type == mission_type;
    }
    if let Some(request) = (message as &dyn std::any::Any).downcast_ref::<MissionRequest>() {
        return request.seq == seq && request.mission_type == mission_type;
    }
    false
}

fn match_message_item(
    message: &dyn crate::mavlink_message::MavlinkMessage,
    seq: u16,
    mission_type: MavMissionType,
) -> bool {
    if let Some(item) = (message as &dyn std::any::Any).downcast_ref::<MissionItemInt>() {
        return item.seq == seq && item.mission_type == mission_type;
    }
    if let Some(item) = (message as &dyn std::any::Any).downcast_ref::<MissionItem>() {
        return item.seq == seq && item.mission_type == mission_type;
    }
    false
}

/// Vehicle-side mission protocol handler.
pub struct MissionServer {
    session: Arc<MavlinkSession>,
    mission_type: MavMissionType,
    items: Arc<Mutex<Vec<MissionItemInt>>>,
    incoming: Arc<Mutex<HashMap<u16, MissionItemInt>>>,
    incoming_count: Arc<Mutex<Option<u16>>>,
    reader: JoinHandle<()>,
}

impl MissionServer {
    pub fn new(
        session: Arc<MavlinkSession>,
        initial_mission: Option<Vec<MissionItemInt>>,
        mission_type: MavMissionType,
    ) -> Self {
        let items = Arc::new(Mutex::new(initial_mission.unwrap_or_default()));
        let incoming = Arc::new(Mutex::new(HashMap::new()));
        let incoming_count = Arc::new(Mutex::new(None));
        let session_reader = Arc::clone(&session);
        let items_reader = Arc::clone(&items);
        let incoming_reader = Arc::clone(&incoming);
        let incoming_count_reader = Arc::clone(&incoming_count);
        let reader = tokio::spawn(async move {
            let mut frames = session_reader.frames();
            while let Some(frame) = frames.next().await {
                let _ = handle_mission_frame(
                    &session_reader,
                    mission_type,
                    &items_reader,
                    &incoming_reader,
                    &incoming_count_reader,
                    frame,
                )
                .await;
            }
        });
        Self {
            session,
            mission_type,
            items,
            incoming,
            incoming_count,
            reader,
        }
    }

    pub fn items(&self) -> Vec<MissionItemInt> {
        self.items.lock().unwrap().clone()
    }

    pub fn replace_mission(&self, items: Vec<MissionItemInt>) {
        *self.items.lock().unwrap() = MissionItems::with_sequential_seq(items);
        self.incoming.lock().unwrap().clear();
        *self.incoming_count.lock().unwrap() = None;
    }

    pub async fn close(self) {
        self.reader.abort();
    }
}

async fn handle_mission_frame(
    session: &MavlinkSession,
    mission_type: MavMissionType,
    items: &Arc<Mutex<Vec<MissionItemInt>>>,
    incoming: &Arc<Mutex<HashMap<u16, MissionItemInt>>>,
    incoming_count: &Arc<Mutex<Option<u16>>>,
    frame: Arc<crate::mavlink_frame::MavlinkFrame>,
) -> Result<(), SessionWaitError> {
    let message = frame.message.as_ref();

    if let Some(count) = (message as &dyn std::any::Any).downcast_ref::<MissionCount>() {
        if !targets_us(session, count.target_system, count.target_component) {
            return Ok(());
        }
        if count.mission_type != mission_type {
            return Ok(());
        }
        *incoming_count.lock().unwrap() = Some(count.count);
        incoming.lock().unwrap().clear();
        if count.count > 0 {
            request_upload_item(session, &frame, mission_type, 0).await?;
        } else {
            send_upload_ack(session, &frame, mission_type).await?;
        }
        return Ok(());
    }

    if let Some(item) = (message as &dyn std::any::Any).downcast_ref::<MissionItemInt>() {
        if !targets_us(session, item.target_system, item.target_component) {
            return Ok(());
        }
        if item.mission_type != mission_type {
            return Ok(());
        }
        store_incoming_item(session, mission_type, items, incoming, incoming_count, &frame, item.clone())
            .await?;
        return Ok(());
    }

    if let Some(item) = (message as &dyn std::any::Any).downcast_ref::<MissionItem>() {
        if !targets_us(session, item.target_system, item.target_component) {
            return Ok(());
        }
        if item.mission_type != mission_type {
            return Ok(());
        }
        store_incoming_item(
            session,
            mission_type,
            items,
            incoming,
            incoming_count,
            &frame,
            MissionItems::from_legacy_item(item),
        )
        .await?;
        return Ok(());
    }

    if let Some(request) = (message as &dyn std::any::Any).downcast_ref::<MissionRequestInt>() {
        if targets_us(session, request.target_system, request.target_component) {
            send_requested_item(session, items, &frame, mission_type, request.seq).await?;
        }
        return Ok(());
    }

    if let Some(request) = (message as &dyn std::any::Any).downcast_ref::<MissionRequest>() {
        if targets_us(session, request.target_system, request.target_component) {
            send_requested_item(session, items, &frame, mission_type, request.seq).await?;
        }
        return Ok(());
    }

    if let Some(request) = (message as &dyn std::any::Any).downcast_ref::<MissionRequestList>() {
        if !targets_us(session, request.target_system, request.target_component) {
            return Ok(());
        }
        if request.mission_type != mission_type {
            return Ok(());
        }
        let count = items.lock().unwrap().len() as u16;
        session
            .send(Box::new(MissionCount {
                count,
                target_system: frame.system_id,
                target_component: frame.component_id,
                mission_type,
            }))
            .await?;
        return Ok(());
    }

    if let Some(clear) = (message as &dyn std::any::Any).downcast_ref::<MissionClearAll>() {
        if !targets_us(session, clear.target_system, clear.target_component) {
            return Ok(());
        }
        if clear.mission_type != mission_type {
            return Ok(());
        }
        items.lock().unwrap().clear();
        incoming.lock().unwrap().clear();
        *incoming_count.lock().unwrap() = None;
        session
            .send(Box::new(MissionAck {
                target_system: frame.system_id,
                target_component: frame.component_id,
                r#type: MavMissionResult::MAV_MISSION_ACCEPTED,
                mission_type,
            }))
            .await?;
    }

    Ok(())
}

async fn store_incoming_item(
    session: &MavlinkSession,
    mission_type: MavMissionType,
    items: &Arc<Mutex<Vec<MissionItemInt>>>,
    incoming: &Arc<Mutex<HashMap<u16, MissionItemInt>>>,
    incoming_count: &Arc<Mutex<Option<u16>>>,
    frame: &Arc<crate::mavlink_frame::MavlinkFrame>,
    item: MissionItemInt,
) -> Result<(), SessionWaitError> {
    incoming.lock().unwrap().insert(item.seq, item.clone());
    let expected = *incoming_count.lock().unwrap();
    let Some(expected) = expected else {
        return Ok(());
    };

    if incoming.lock().unwrap().len() < expected as usize {
        request_upload_item(session, frame, mission_type, item.seq + 1).await?;
        return Ok(());
    }

    let mut ordered = Vec::with_capacity(expected as usize);
    for seq in 0..expected {
        let Some(next) = incoming.lock().unwrap().get(&seq).cloned() else {
            return Ok(());
        };
        ordered.push(next);
    }
    *items.lock().unwrap() = ordered;
    incoming.lock().unwrap().clear();
    *incoming_count.lock().unwrap() = None;
    send_upload_ack(session, frame, mission_type).await
}

async fn request_upload_item(
    session: &MavlinkSession,
    frame: &Arc<crate::mavlink_frame::MavlinkFrame>,
    mission_type: MavMissionType,
    seq: u16,
) -> Result<(), SessionWaitError> {
    session
        .send(Box::new(MissionRequestInt {
            seq,
            target_system: frame.system_id,
            target_component: frame.component_id,
            mission_type,
        }))
        .await
}

async fn send_upload_ack(
    session: &MavlinkSession,
    frame: &Arc<crate::mavlink_frame::MavlinkFrame>,
    mission_type: MavMissionType,
) -> Result<(), SessionWaitError> {
    session
        .send(Box::new(MissionAck {
            target_system: frame.system_id,
            target_component: frame.component_id,
            r#type: MavMissionResult::MAV_MISSION_ACCEPTED,
            mission_type,
        }))
        .await
}

async fn send_requested_item(
    session: &MavlinkSession,
    items: &Arc<Mutex<Vec<MissionItemInt>>>,
    frame: &Arc<crate::mavlink_frame::MavlinkFrame>,
    mission_type: MavMissionType,
    seq: u16,
) -> Result<(), SessionWaitError> {
    let maybe_item = {
        let items_guard = items.lock().unwrap();
        if seq as usize >= items_guard.len() {
            None
        } else {
            Some(items_guard[seq as usize].clone())
        }
    };
    if maybe_item.is_none() {
        return session
            .send(Box::new(MissionAck {
                target_system: frame.system_id,
                target_component: frame.component_id,
                r#type: MavMissionResult::MAV_MISSION_INVALID_SEQUENCE,
                mission_type,
            }))
            .await;
    }
    session.send(Box::new(maybe_item.unwrap())).await
}

fn targets_us(session: &MavlinkSession, target_system: u8, target_component: u8) -> bool {
    if target_system != session.system_id() && target_system != 0 {
        return false;
    }
    if target_component != session.component_id()
        && target_component != MavComponent::MAV_COMP_ID_ALL as u8
    {
        return false;
    }
    true
}
