//! MAVLink parameter protocol client and server.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use tokio::task::JoinHandle;

use crate::{
    MavComponent, MavParamType, ParamRequestList, ParamRequestRead, ParamSet, ParamValue,
};

use super::mavlink_cancellation::MavlinkCancellationToken;
use super::mavlink_session::{MavlinkSession, SessionWaitError};
use super::param_codec::ParamCodec;

/// Decoded onboard parameter entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamEntry {
    pub id: String,
    pub value: f64,
    pub param_type: MavParamType,
    pub index: u16,
    pub count: u16,
}

impl ParamEntry {
    pub fn from_param_value(message: &ParamValue) -> Self {
        Self {
            id: ParamCodec::param_id_to_string(&message.param_id),
            value: ParamCodec::decode_value(message.param_value, message.param_type),
            param_type: message.param_type,
            index: message.param_index,
            count: message.param_count,
        }
    }
}

pub type ParamProgressCallback = dyn Fn(&ParamEntry, usize, u16) + Send + Sync;

/// GCS-side MAVLink parameter protocol client.
pub struct ParameterProtocol {
    session: Arc<MavlinkSession>,
    target_system: u8,
    target_component: u8,
    idle_timeout: Duration,
    request_timeout: Duration,
    cache: Mutex<HashMap<String, ParamEntry>>,
}

impl ParameterProtocol {
    pub fn new(
        session: Arc<MavlinkSession>,
        target_system: u8,
        target_component: u8,
        idle_timeout: Duration,
        request_timeout: Duration,
    ) -> Self {
        Self {
            session,
            target_system,
            target_component,
            idle_timeout,
            request_timeout,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn cache(&self) -> HashMap<String, ParamEntry> {
        self.cache.lock().unwrap().clone()
    }

    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    fn remember(&self, entry: ParamEntry) {
        self.cache.lock().unwrap().insert(entry.id.clone(), entry);
    }

    pub fn type_for_name(&self, name: &str) -> Option<MavParamType> {
        self.cache.lock().unwrap().get(name).map(|entry| entry.param_type)
    }

    pub async fn fetch_all(
        &self,
        on_progress: Option<&ParamProgressCallback>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<Vec<ParamEntry>, SessionWaitError> {
        let mut entries = Vec::new();
        let mut stream = self.fetch_all_stream(cancel).await;
        while let Some(entry) = stream.fetch_next().await {
            entries.push(entry.clone());
            if let Some(callback) = on_progress {
                callback(&entry, entries.len(), entry.count);
            }
        }
        Ok(entries)
    }

    pub async fn fetch_all_stream(
        &self,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> ParameterFetchStream<'_> {
        if let Some(token) = cancel {
            let _ = token.throw_if_cancelled();
        }

        let _ = self
            .session
            .send(Box::new(ParamRequestList {
                target_system: self.target_system,
                target_component: self.target_component,
            }))
            .await;

        ParameterFetchStream {
            protocol: self,
            cancel: cancel.cloned(),
            expected_count: None,
            seen_indices: HashSet::new(),
            finished: false,
        }
    }

    pub async fn read_by_name(
        &self,
        name: &str,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamEntry, SessionWaitError> {
        self.read(Some(name), -1, cancel).await
    }

    pub async fn read_by_index(
        &self,
        index: i16,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamEntry, SessionWaitError> {
        self.read(None, index, cancel).await
    }

    pub async fn read(
        &self,
        param_id: Option<&str>,
        param_index: i16,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamEntry, SessionWaitError> {
        if param_id.is_none() && param_index < 0 {
            return Err(SessionWaitError::Closed);
        }

        self.session
            .send(Box::new(ParamRequestRead {
                param_index,
                target_system: self.target_system,
                target_component: self.target_component,
                param_id: ParamCodec::param_id_from_string(param_id.unwrap_or("")),
            }))
            .await?;

        let value = self
            .session
            .wait_for_message_type::<ParamValue>(
                Some(self.target_system),
                None,
                self.request_timeout,
                cancel,
            )
            .await?;

        let entry = ParamEntry::from_param_value(&value);
        self.remember(entry.clone());
        Ok(entry)
    }

    pub async fn write(
        &self,
        name: &str,
        value: f64,
        param_type: MavParamType,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamEntry, SessionWaitError> {
        self.session
            .send(Box::new(ParamSet {
                param_value: ParamCodec::encode_value(value, param_type),
                target_system: self.target_system,
                target_component: self.target_component,
                param_id: ParamCodec::param_id_from_string(name),
                param_type,
            }))
            .await?;

        let name = name.to_string();
        let ack = self
            .session
            .wait_for_message(
                move |message| {
                    if let Some(param) = (message as &dyn std::any::Any).downcast_ref::<ParamValue>() {
                        return ParamCodec::param_id_to_string(&param.param_id) == name;
                    }
                    false
                },
                Some(self.target_system),
                None,
                self.request_timeout,
                cancel,
            )
            .await?;

        let param_value = (ack.as_ref() as &dyn std::any::Any)
            .downcast_ref::<ParamValue>()
            .expect("predicate ensures ParamValue");
        let entry = ParamEntry::from_param_value(param_value);
        self.remember(entry.clone());
        Ok(entry)
    }

    pub async fn write_by_name(
        &self,
        name: &str,
        value: f64,
        param_type: Option<MavParamType>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamEntry, SessionWaitError> {
        let resolved = param_type
            .or_else(|| self.type_for_name(name))
            .unwrap_or(MavParamType::MAV_PARAM_TYPE_REAL32);
        self.write(name, value, resolved, cancel).await
    }

    async fn next_param_value(
        &self,
        expected_count: Option<u16>,
        seen_indices: &HashSet<u16>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<ParamValue, SessionWaitError> {
        let timeout = if expected_count.is_none() {
            self.request_timeout
        } else {
            self.idle_timeout
        };

        let seen = seen_indices.clone();
        self.session
            .wait_for_message(
                move |message| {
                    if let Some(param) = (message as &dyn std::any::Any).downcast_ref::<ParamValue>() {
                        return !seen.contains(&param.param_index);
                    }
                    false
                },
                Some(self.target_system),
                None,
                timeout,
                cancel,
            )
            .await
            .and_then(|message| {
                (message.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<ParamValue>()
                    .cloned()
                    .ok_or(SessionWaitError::Closed)
            })
    }
}

pub struct ParameterFetchStream<'a> {
    protocol: &'a ParameterProtocol,
    cancel: Option<MavlinkCancellationToken>,
    expected_count: Option<u16>,
    seen_indices: HashSet<u16>,
    finished: bool,
}

impl<'a> ParameterFetchStream<'a> {
    pub async fn fetch_next(&mut self) -> Option<ParamEntry> {
        if self.finished {
            return None;
        }

        let mut retries = 0;
        let max_retries = 5;

        loop {
            if let Some(token) = self.cancel.as_ref() {
                if token.throw_if_cancelled().is_err() {
                    self.finished = true;
                    return None;
                }
            }

            match self
                .protocol
                .next_param_value(self.expected_count, &self.seen_indices, self.cancel.as_ref())
                .await
            {
                Ok(value) => {
                    self.seen_indices.insert(value.param_index);
                    if self.expected_count.is_none() {
                        self.expected_count = Some(value.param_count);
                    }
                    let entry = ParamEntry::from_param_value(&value);
                    self.protocol.remember(entry.clone());

                    if let Some(expected) = self.expected_count {
                        if self.seen_indices.len() >= expected as usize {
                            self.finished = true;
                        }
                    }
                    return Some(entry);
                }
                Err(SessionWaitError::Timeout(_)) => {
                    if let Some(expected) = self.expected_count {
                        retries += 1;
                        if retries > max_retries {
                            self.finished = true;
                            return None;
                        }

                        let mut missing_index = -1;
                        for i in 0..expected {
                            if !self.seen_indices.contains(&i) {
                                missing_index = i as i16;
                                break;
                            }
                        }

                        if missing_index >= 0 {
                            let _ = self.protocol.session.send(Box::new(ParamRequestRead {
                                param_index: missing_index,
                                target_system: self.protocol.target_system,
                                target_component: self.protocol.target_component,
                                param_id: ParamCodec::param_id_from_string(""),
                            })).await;
                        } else {
                            self.finished = true;
                            return None;
                        }
                    } else {
                        self.finished = true;
                        return None;
                    }
                }
                Err(_) => {
                    self.finished = true;
                    return None;
                }
            }
        }
    }
}

#[derive(Clone)]
struct StoredParam {
    value: f64,
    param_type: MavParamType,
}

/// Vehicle-side parameter store handler.
pub struct ParameterServer {
    session: Arc<MavlinkSession>,
    values: Arc<Mutex<HashMap<String, StoredParam>>>,
    reader: JoinHandle<()>,
}

impl ParameterServer {
    pub fn new(
        session: Arc<MavlinkSession>,
        initial_values: Option<HashMap<String, StoredParam>>,
    ) -> Self {
        let values = Arc::new(Mutex::new(initial_values.unwrap_or_default()));
        let session_reader = Arc::clone(&session);
        let values_reader = Arc::clone(&values);
        let reader = tokio::spawn(async move {
            let mut frames = session_reader.frames();
            while let Some(frame) = frames.next().await {
                let _ = handle_param_frame(&session_reader, &values_reader, frame).await;
            }
        });
        Self {
            session,
            values,
            reader,
        }
    }

    pub fn from_typed(
        session: Arc<MavlinkSession>,
        initial_values: HashMap<String, (f64, MavParamType)>,
    ) -> Self {
        let mapped = initial_values
            .into_iter()
            .map(|(name, (value, param_type))| {
                (
                    name,
                    StoredParam {
                        value,
                        param_type,
                    },
                )
            })
            .collect();
        Self::new(session, Some(mapped))
    }

    pub fn values(&self) -> HashMap<String, (f64, MavParamType)> {
        self.values
            .lock()
            .unwrap()
            .iter()
            .map(|(name, entry)| (name.clone(), (entry.value, entry.param_type)))
            .collect()
    }

    pub fn set(&self, name: &str, value: f64, param_type: MavParamType) {
        self.values.lock().unwrap().insert(
            name.to_string(),
            StoredParam {
                value,
                param_type,
            },
        );
    }

    pub async fn close(self) {
        self.reader.abort();
    }
}

async fn handle_param_frame(
    session: &MavlinkSession,
    values: &Arc<Mutex<HashMap<String, StoredParam>>>,
    frame: Arc<crate::mavlink_frame::MavlinkFrame>,
) -> Result<(), SessionWaitError> {
    if let Some(request) = (frame.message.as_ref() as &dyn std::any::Any).downcast_ref::<ParamRequestList>()
    {
        if request.target_system != session.system_id()
            && request.target_system != MavComponent::MAV_COMP_ID_ALL as u8
        {
            return Ok(());
        }
        broadcast_all(session, values).await?;
        return Ok(());
    }

    if let Some(request) = (frame.message.as_ref() as &dyn std::any::Any).downcast_ref::<ParamRequestRead>()
    {
        if request.target_system != session.system_id()
            && request.target_system != MavComponent::MAV_COMP_ID_ALL as u8
        {
            return Ok(());
        }
        if let Some((name, entry, index)) = resolve_read(values, request) {
            send_value(session, values, &name, &entry, index).await?;
        }
        return Ok(());
    }

    if let Some(param_set) = (frame.message.as_ref() as &dyn std::any::Any).downcast_ref::<ParamSet>() {
        if param_set.target_system != session.system_id() {
            return Ok(());
        }
        let name = ParamCodec::param_id_to_string(&param_set.param_id);
        let entry = StoredParam {
            value: ParamCodec::decode_value(param_set.param_value, param_set.param_type),
            param_type: param_set.param_type,
        };
        values.lock().unwrap().insert(name.clone(), entry.clone());
        let index = index_of(values, &name);
        send_value(session, values, &name, &entry, index).await?;
    }

    Ok(())
}

async fn broadcast_all(
    session: &MavlinkSession,
    values: &Arc<Mutex<HashMap<String, StoredParam>>>,
) -> Result<(), SessionWaitError> {
    let names: Vec<_> = values.lock().unwrap().keys().cloned().collect();
    for (index, name) in names.iter().enumerate() {
        let entry = {
            let guard = values.lock().unwrap();
            guard.get(name).unwrap().clone()
        };
        send_value(session, values, name, &entry, index).await?;
    }
    Ok(())
}

async fn send_value(
    session: &MavlinkSession,
    values: &Arc<Mutex<HashMap<String, StoredParam>>>,
    name: &str,
    entry: &StoredParam,
    index: usize,
) -> Result<(), SessionWaitError> {
    let param_count = values.lock().unwrap().len() as u16;
    session
        .send(Box::new(ParamValue {
            param_value: ParamCodec::encode_value(entry.value, entry.param_type),
            param_count,
            param_index: index as u16,
            param_id: ParamCodec::param_id_from_string(name),
            param_type: entry.param_type,
        }))
        .await
}

fn resolve_read(
    values: &Arc<Mutex<HashMap<String, StoredParam>>>,
    request: &ParamRequestRead,
) -> Option<(String, StoredParam, usize)> {
    if request.param_index >= 0 {
        let names: Vec<_> = values.lock().unwrap().keys().cloned().collect();
        let index = request.param_index as usize;
        if index >= names.len() {
            return None;
        }
        let name = names[index].clone();
        let entry = values.lock().unwrap().get(&name)?.clone();
        return Some((name, entry, index));
    }

    let name = ParamCodec::param_id_to_string(&request.param_id);
    let entry = values.lock().unwrap().get(&name)?.clone();
    let index = index_of(values, &name);
    Some((name, entry, index))
}

fn index_of(values: &Arc<Mutex<HashMap<String, StoredParam>>>, name: &str) -> usize {
    values
        .lock()
        .unwrap()
        .keys()
        .position(|key| key == name)
        .unwrap_or(0)
}
