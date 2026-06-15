//! Virtual telemetry request for the `rt_rc` dialect.

mod common;

use common::*;

fn main() {
    let dialect = MavlinkDialectRtRc;

    let set_interval = CommandLong {
        param1: Attitude::MSG_ID as f32,
        param2: 100_000.0,
        param3: 0.0,
        param4: 0.0,
        param5: 0.0,
        param6: 0.0,
        param7: 0.0,
        command: MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        confirmation: 0,
    };
    let interval_frame = frame_from_gcs(Box::new(set_interval.clone()), 1);
    log_frame("GCS ->", &interval_frame);
    if let Some(parsed) = round_trip_message(&dialect, &set_interval) {
        if let Some(parsed_interval) = downcast_message::<CommandLong>(parsed.as_ref()) {
            println!(
                "  SET_MESSAGE_INTERVAL msgId={} interval_us={}",
                parsed_interval.param1 as u32, parsed_interval.param2 as u32
            );
        }
    }

    let request_once = CommandLong {
        param1: Attitude::MSG_ID as f32,
        param2: 0.0,
        param3: 0.0,
        param4: 0.0,
        param5: 0.0,
        param6: 0.0,
        param7: 0.0,
        command: MavCmd::MAV_CMD_REQUEST_MESSAGE,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        confirmation: 0,
    };
    let once_frame = frame_from_gcs(Box::new(request_once.clone()), 2);
    log_frame("GCS ->", &once_frame);
    let _ = round_trip_message(&dialect, &request_once);

    let attitude = Attitude {
        time_boot_ms: 12_345,
        roll: 0.01,
        pitch: -0.02,
        yaw: 1.57,
        rollspeed: 0.0,
        pitchspeed: 0.0,
        yawspeed: 0.0,
    };
    let telemetry_frame = frame_from_drone(Box::new(attitude.clone()), 3);
    log_frame("Drone ->", &telemetry_frame);
    if let Some(parsed) = round_trip_message(&dialect, &attitude) {
        if let Some(parsed_attitude) = downcast_message::<Attitude>(parsed.as_ref()) {
            println!(
                "  ATTITUDE roll={} pitch={} yaw={}",
                parsed_attitude.roll, parsed_attitude.pitch, parsed_attitude.yaw
            );
        }
    }
}
