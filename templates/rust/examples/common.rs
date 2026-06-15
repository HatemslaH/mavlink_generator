//! Shared helpers for MAVLink Rust examples.

use std::any::Any;

pub use mavlink::*;

/// Ground control station identity (MAVLink convention).
pub const GCS_SYSTEM_ID: u8 = 255;
pub const GCS_COMPONENT_ID: u8 = 190;

/// Simulated autopilot identity.
pub const DRONE_SYSTEM_ID: u8 = 1;
pub const DRONE_COMPONENT_ID: u8 = 1;

pub fn frame_from_gcs(message: Box<dyn MavlinkMessage>, sequence: u8) -> MavlinkFrame {
    MavlinkFrame::v2(sequence, GCS_SYSTEM_ID, GCS_COMPONENT_ID, message)
}

pub fn frame_from_drone(message: Box<dyn MavlinkMessage>, sequence: u8) -> MavlinkFrame {
    MavlinkFrame::v2(sequence, DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, message)
}

pub fn param_id_from_string(name: &str) -> [u8; 16] {
    let mut param_id = [0u8; 16];
    for (i, byte) in name.bytes().take(16).enumerate() {
        param_id[i] = byte;
    }
    param_id
}

pub fn param_id_to_string(param_id: &[u8; 16]) -> String {
    let end = param_id.iter().position(|&b| b == 0).unwrap_or(param_id.len());
    String::from_utf8_lossy(&param_id[..end]).into_owned()
}

pub fn log_frame(direction: &str, frame: &MavlinkFrame) {
    println!(
        "{} msgId={} sys={} comp={}",
        direction,
        frame.message.mavlink_message_id(),
        frame.system_id,
        frame.component_id
    );
}

pub fn downcast_message<T: Any>(message: &dyn MavlinkMessage) -> Option<&T> {
    (message as &dyn Any).downcast_ref::<T>()
}

pub fn round_trip_message(
    dialect: &dyn MavlinkDialect,
    message: &dyn MavlinkMessage,
) -> Option<Box<dyn MavlinkMessage>> {
    dialect.parse(message.mavlink_message_id(), &message.serialize())
}
