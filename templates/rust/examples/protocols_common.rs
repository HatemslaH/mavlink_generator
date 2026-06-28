//! Shared helpers for protocol-class MAVLink examples.

use std::sync::Arc;

pub use mavlink::mavlink_dialect::MavlinkDialect;
pub use mavlink::mavlink_version::MavlinkVersion;
pub use mavlink::protocols::{
    MavlinkLink, MavlinkSession, SessionWaitError, VirtualMavlinkBus,
};

/// Ground control station identity (MAVLink convention).
pub const GCS_SYSTEM_ID: u8 = 255;
pub const GCS_COMPONENT_ID: u8 = 190;

/// Simulated autopilot identity.
pub const DRONE_SYSTEM_ID: u8 = 1;
pub const DRONE_COMPONENT_ID: u8 = 1;

pub struct VirtualLink {
    pub bus: Arc<VirtualMavlinkBus>,
    pub gcs: Arc<MavlinkSession>,
    pub drone: Arc<MavlinkSession>,
    pub dialect: Arc<dyn MavlinkDialect + Send + Sync>,
}

pub fn create_virtual_link(
    dialect: Arc<dyn MavlinkDialect + Send + Sync>,
) -> VirtualLink {
    let bus = VirtualMavlinkBus::new();
    let gcs_link = bus.create_endpoint();
    let drone_link = bus.create_endpoint();

    let gcs = Arc::new(MavlinkSession::new(
        dialect.clone(),
        gcs_link,
        GCS_SYSTEM_ID,
        GCS_COMPONENT_ID,
        MavlinkVersion::V2,
    ));

    let drone = Arc::new(MavlinkSession::new(
        dialect.clone(),
        drone_link,
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        MavlinkVersion::V2,
    ));

    VirtualLink {
        bus,
        gcs,
        drone,
        dialect,
    }
}

pub async fn close_virtual_link(link: VirtualLink) -> Result<(), SessionWaitError> {
    link.gcs.close().await?;
    link.drone.close().await?;
    link.bus.close_all().await;
    Ok(())
}
