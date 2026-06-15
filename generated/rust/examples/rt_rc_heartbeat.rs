//! Example for the `rt_rc` dialect: serialize a Heartbeat frame and parse it back.

mod common;

use common::*;

fn main() {
    let dialect = MavlinkDialectRtRc;

    let heartbeat = Heartbeat {
        custom_mode: 0,
        r#type: MavType::MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot::MAV_AUTOPILOT_PX4,
        base_mode: 0,
        system_status: MavState::MAV_STATE_ACTIVE,
        mavlink_version: dialect.version(),
    };

    let frame = frame_from_gcs(Box::new(heartbeat.clone()), 0);
    let wire = frame.serialize();
    log_frame("GCS ->", &frame);
    println!("Serialized HEARTBEAT ({} bytes)", wire.len());

    if let Some(parsed) = round_trip_message(&dialect, &heartbeat) {
        if let Some(heartbeat) = downcast_message::<Heartbeat>(parsed.as_ref()) {
            println!(
                "Parsed HEARTBEAT type={:?} status={:?}",
                heartbeat.r#type, heartbeat.system_status
            );
        }
    }
}
