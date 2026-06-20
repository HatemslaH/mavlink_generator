//! Shared MAVLink GCS state for the interactive SITL example.

use mavlink::protocols::{
    CommandProtocol, HeartbeatMonitor, HeartbeatPublisher, MavlinkCancellationToken,
    MavlinkGcs, MavlinkNode, MavlinkSession, MavlinkVehicleClient, MissionProtocol,
    ParameterProtocol,
};

/// Ground control station identity (MAVLink convention).
pub const GCS_SYSTEM_ID: u8 = 255;
pub const GCS_COMPONENT_ID: u8 = 190;

/// Shared MAVLink GCS state for the interactive SITL example.
pub struct GcsContext {
    pub gcs: MavlinkGcs,
    pub vehicle: MavlinkNode,
    pub client: MavlinkVehicleClient,
    /// Cancels in-flight parameter/mission operations (`cancel` CLI command).
    pub operation_cancel: Option<MavlinkCancellationToken>,
}

impl GcsContext {
    pub fn new(gcs: MavlinkGcs, vehicle: MavlinkNode, client: MavlinkVehicleClient) -> Self {
        Self {
            gcs,
            vehicle,
            client,
            operation_cancel: None,
        }
    }

    pub fn session(&self) -> &MavlinkSession {
        &self.gcs.session
    }

    pub fn heartbeat_monitor(&self) -> &HeartbeatMonitor {
        &self.gcs.heartbeat_monitor
    }

    pub fn heartbeat_publisher(&self) -> &HeartbeatPublisher {
        &self.gcs.heartbeat_publisher
    }

    pub fn parameters(&self) -> &ParameterProtocol {
        &self.client.parameters
    }

    pub fn mission(&self) -> &MissionProtocol {
        &self.client.mission
    }

    pub fn command(&self) -> &CommandProtocol {
        &self.client.command
    }

    pub fn target_system(&self) -> u8 {
        self.vehicle.system_id
    }

    pub fn target_component(&self) -> u8 {
        self.vehicle.component_id
    }
}
