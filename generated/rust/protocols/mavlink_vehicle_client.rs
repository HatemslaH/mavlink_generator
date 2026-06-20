//! GCS bootstrap and vehicle protocol facade.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::mavlink_dialect::MavlinkDialect;
use crate::mavlink_version::MavlinkVersion;

use super::command_protocol::CommandProtocol;
use super::heartbeat_protocol::{HeartbeatMonitor, HeartbeatPublisher, HeartbeatTemplates, MavlinkNode};
use super::mavlink_link::MavlinkLink;
use super::mavlink_session::{MavlinkSession, SessionWaitError};
use super::mission_protocol::MissionProtocol;
use super::parameter_protocol::ParameterProtocol;

/// Protocol clients bound to a single remote MAVLink vehicle.
pub struct MavlinkVehicleClient {
    pub session: Arc<MavlinkSession>,
    pub vehicle: MavlinkNode,
    pub parameters: ParameterProtocol,
    pub mission: MissionProtocol,
    pub command: CommandProtocol,
}

impl MavlinkVehicleClient {
    pub fn new(
        session: Arc<MavlinkSession>,
        vehicle: MavlinkNode,
        parameter_request_timeout: Duration,
        parameter_idle_timeout: Duration,
        mission_item_timeout: Duration,
        mission_operation_timeout: Duration,
        command_timeout: Duration,
    ) -> Self {
        Self {
            parameters: ParameterProtocol::new(
                Arc::clone(&session),
                vehicle.system_id,
                vehicle.component_id,
                parameter_idle_timeout,
                parameter_request_timeout,
            ),
            mission: MissionProtocol::new(
                Arc::clone(&session),
                vehicle.system_id,
                vehicle.component_id,
                mission_item_timeout,
                mission_operation_timeout,
            ),
            command: CommandProtocol::new(
                Arc::clone(&session),
                vehicle.system_id,
                vehicle.component_id,
                command_timeout,
            ),
            session,
            vehicle,
        }
    }

    pub fn target_system(&self) -> u8 {
        self.vehicle.system_id
    }

    pub fn target_component(&self) -> u8 {
        self.vehicle.component_id
    }
}

/// Ground control station bootstrap: session, heartbeat publisher, and monitor.
pub struct MavlinkGcs {
    pub session: Arc<MavlinkSession>,
    pub heartbeat_publisher: HeartbeatPublisher,
    pub heartbeat_monitor: HeartbeatMonitor,
}

impl MavlinkGcs {
    pub fn new(
        session: Arc<MavlinkSession>,
        heartbeat_publisher: HeartbeatPublisher,
        heartbeat_monitor: HeartbeatMonitor,
    ) -> Self {
        Self {
            session,
            heartbeat_publisher,
            heartbeat_monitor,
        }
    }

    pub fn start(&self) {
        self.heartbeat_monitor.start();
        self.heartbeat_publisher.start();
    }

    pub async fn stop_heartbeats(&self) {
        self.heartbeat_publisher.stop();
        self.heartbeat_monitor.stop().await;
    }

    pub async fn wait_for_vehicle(
        &self,
        exclude_system_ids: Option<HashSet<u8>>,
        timeout: Duration,
    ) -> Result<MavlinkVehicleClient, SessionWaitError> {
        let vehicle = self
            .heartbeat_monitor
            .wait_for_vehicle(exclude_system_ids, timeout, None)
            .await?;
        Ok(self.vehicle_client(vehicle))
    }

    pub fn vehicle_client(&self, vehicle: MavlinkNode) -> MavlinkVehicleClient {
        MavlinkVehicleClient::new(
            Arc::clone(&self.session),
            vehicle,
            Duration::from_secs(10),
            Duration::from_secs(2),
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(10),
        )
    }

    pub fn connect(
        dialect: Arc<dyn MavlinkDialect + Send + Sync>,
        link: Arc<dyn MavlinkLink>,
        system_id: u8,
        component_id: u8,
        heartbeat_interval: Duration,
        heartbeat_timeout: Duration,
    ) -> Self {
        let session = Arc::new(MavlinkSession::new(
            dialect.clone(),
            link,
            system_id,
            component_id,
            MavlinkVersion::V2,
        ));

        Self::new(
            Arc::clone(&session),
            HeartbeatPublisher::new(
                Arc::clone(&session),
                HeartbeatTemplates::gcs(dialect.version()),
                heartbeat_interval,
            ),
            HeartbeatMonitor::new(
                session,
                heartbeat_timeout,
                None,
                None,
            ),
        )
    }

    pub async fn close(self) -> Result<(), SessionWaitError> {
        self.stop_heartbeats().await;
        self.session.close().await
    }
}
