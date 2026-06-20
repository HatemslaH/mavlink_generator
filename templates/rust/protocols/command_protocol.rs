//! MAVLink command protocol client and server.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::task::JoinHandle;

use crate::{CommandAck, CommandInt, CommandLong, MavCmd, MavResult};

use super::mavlink_cancellation::MavlinkCancellationToken;
use super::mavlink_session::{MavlinkSession, SessionWaitError};

/// GCS-side MAVLink command protocol client.
pub struct CommandProtocol {
    session: Arc<MavlinkSession>,
    target_system: u8,
    target_component: u8,
    default_timeout: Duration,
}

impl CommandProtocol {
    pub fn new(
        session: Arc<MavlinkSession>,
        target_system: u8,
        target_component: u8,
        default_timeout: Duration,
    ) -> Self {
        Self {
            session,
            target_system,
            target_component,
            default_timeout,
        }
    }

    pub async fn send_long(
        &self,
        command: CommandLong,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        let cmd = command.command;
        self.session.send(Box::new(command)).await?;
        self.wait_for_ack(cmd, timeout, cancel).await
    }

    pub async fn send_int(
        &self,
        command: CommandInt,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        let cmd = command.command;
        self.session.send(Box::new(command)).await?;
        self.wait_for_ack(cmd, timeout, cancel).await
    }

    pub async fn command_long(
        &self,
        command: MavCmd,
        param1: f32,
        param2: f32,
        param3: f32,
        param4: f32,
        param5: f32,
        param6: f32,
        param7: f32,
        confirmation: u8,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.send_long(
            CommandLong {
                param1,
                param2,
                param3,
                param4,
                param5,
                param6,
                param7,
                command,
                target_system: self.target_system,
                target_component: self.target_component,
                confirmation,
            },
            timeout,
            cancel,
        )
        .await
    }

    pub async fn request_message(
        &self,
        message_id: u32,
        param2: f32,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_REQUEST_MESSAGE,
            message_id as f32,
            param2,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn set_message_interval(
        &self,
        message_id: u32,
        interval_us: u32,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
            message_id as f32,
            interval_us as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn stop_message_interval(
        &self,
        message_id: u32,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.set_message_interval(message_id, 0, timeout, cancel)
            .await
    }

    pub async fn set_mission_current(
        &self,
        sequence: u16,
        reset_mission: bool,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_DO_SET_MISSION_CURRENT,
            f32::from(sequence),
            if reset_mission { 1.0 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn arm(
        &self,
        force: bool,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_COMPONENT_ARM_DISARM,
            1.0,
            if force { 21196.0 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn disarm(
        &self,
        force: bool,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_COMPONENT_ARM_DISARM,
            0.0,
            if force { 21196.0 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn takeoff(
        &self,
        altitude: f32,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_NAV_TAKEOFF,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            altitude,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn land(
        &self,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_NAV_LAND,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn return_to_launch(
        &self,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.command_long(
            MavCmd::MAV_CMD_NAV_RETURN_TO_LAUNCH,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            timeout,
            cancel,
        )
        .await
    }

    pub async fn wait_for_ack(
        &self,
        command: MavCmd,
        timeout: Option<Duration>,
        cancel: Option<&MavlinkCancellationToken>,
    ) -> Result<CommandAck, SessionWaitError> {
        self.session
            .wait_for_message(
                move |message| {
                    if let Some(ack) = (message as &dyn std::any::Any).downcast_ref::<CommandAck>() {
                        return ack.command == command;
                    }
                    false
                },
                Some(self.target_system),
                None,
                timeout.unwrap_or(self.default_timeout),
                cancel,
            )
            .await
            .and_then(|message| {
                (message.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<CommandAck>()
                    .cloned()
                    .ok_or(SessionWaitError::Closed)
            })
    }
}

pub type CommandLongHandler =
    dyn Fn(CommandLong) -> std::pin::Pin<Box<dyn std::future::Future<Output = MavResult> + Send>>
    + Send
    + Sync;
pub type CommandIntHandler =
    dyn Fn(CommandInt) -> std::pin::Pin<Box<dyn std::future::Future<Output = MavResult> + Send>>
    + Send
    + Sync;

/// Vehicle-side command handler.
pub struct CommandServer {
    session: Arc<MavlinkSession>,
    on_command_long: Option<Arc<CommandLongHandler>>,
    on_command_int: Option<Arc<CommandIntHandler>>,
    reader: JoinHandle<()>,
}

impl CommandServer {
    pub fn new(
        session: Arc<MavlinkSession>,
        on_command_long: Option<Arc<CommandLongHandler>>,
        on_command_int: Option<Arc<CommandIntHandler>>,
    ) -> Self {
        let session_reader = Arc::clone(&session);
        let long_handler = on_command_long.clone();
        let int_handler = on_command_int.clone();
        let reader = tokio::spawn(async move {
            let mut frames = session_reader.frames();
            while let Some(frame) = frames.next().await {
                let _ = handle_command_frame(&session_reader, long_handler.as_deref(), int_handler.as_deref(), frame)
                    .await;
            }
        });
        Self {
            session,
            on_command_long,
            on_command_int,
            reader,
        }
    }

    pub async fn close(self) {
        self.reader.abort();
    }
}

async fn handle_command_frame(
    session: &MavlinkSession,
    on_command_long: Option<&CommandLongHandler>,
    on_command_int: Option<&CommandIntHandler>,
    frame: Arc<crate::mavlink_frame::MavlinkFrame>,
) -> Result<(), SessionWaitError> {
    if let Some(command) = (frame.message.as_ref() as &dyn std::any::Any).downcast_ref::<CommandLong>() {
        if command.target_system != session.system_id() {
            return Ok(());
        }
        let result = if let Some(handler) = on_command_long {
            handler(command.clone()).await
        } else {
            MavResult::MAV_RESULT_ACCEPTED
        };
        send_ack(session, &frame, command.command, result).await?;
        return Ok(());
    }

    if let Some(command) = (frame.message.as_ref() as &dyn std::any::Any).downcast_ref::<CommandInt>() {
        if command.target_system != session.system_id() {
            return Ok(());
        }
        let result = if let Some(handler) = on_command_int {
            handler(command.clone()).await
        } else {
            MavResult::MAV_RESULT_ACCEPTED
        };
        send_ack(session, &frame, command.command, result).await?;
    }

    Ok(())
}

async fn send_ack(
    session: &MavlinkSession,
    frame: &Arc<crate::mavlink_frame::MavlinkFrame>,
    command: MavCmd,
    result: MavResult,
) -> Result<(), SessionWaitError> {
    session
        .send(Box::new(CommandAck {
            command,
            result,
            progress: 0,
            result_param2: 0,
            target_system: frame.system_id,
            target_component: frame.component_id,
        }))
        .await
}
