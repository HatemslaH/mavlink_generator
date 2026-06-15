//! Virtual mission upload for the `rt_rc` dialect.

mod common;

use common::*;

fn main() {
    let dialect = MavlinkDialectRtRc;
    let mission_type = MavMissionType::MAV_MISSION_TYPE_MISSION;

    let mission_items = [
        MissionItem {
            param1: 0.0,
            param2: 2.0,
            param3: 0.0,
            param4: 0.0,
            x: 47.397_742,
            y: 8.545_594,
            z: 50.0,
            seq: 0,
            command: MavCmd::MAV_CMD_NAV_WAYPOINT,
            target_system: DRONE_SYSTEM_ID,
            target_component: DRONE_COMPONENT_ID,
            frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current: 0,
            autocontinue: 1,
            mission_type,
        },
        MissionItem {
            param1: 0.0,
            param2: 2.0,
            param3: 0.0,
            param4: 0.0,
            x: 47.398_000,
            y: 8.546_000,
            z: 50.0,
            seq: 1,
            command: MavCmd::MAV_CMD_NAV_WAYPOINT,
            target_system: DRONE_SYSTEM_ID,
            target_component: DRONE_COMPONENT_ID,
            frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current: 0,
            autocontinue: 1,
            mission_type,
        },
    ];

    let count = MissionCount {
        count: mission_items.len() as u16,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        mission_type,
    };
    let count_frame = frame_from_gcs(Box::new(count.clone()), 1);
    log_frame("GCS ->", &count_frame);
    let _ = round_trip_message(&dialect, &count);

    for (seq, item) in mission_items.iter().enumerate() {
        let request = MissionRequest {
            seq: seq as u16,
            target_system: GCS_SYSTEM_ID,
            target_component: GCS_COMPONENT_ID,
            mission_type,
        };
        let request_frame = frame_from_drone(Box::new(request.clone()), (seq + 10) as u8);
        log_frame("Drone ->", &request_frame);
        let _ = round_trip_message(&dialect, &request);

        let item_frame = frame_from_gcs(Box::new(item.clone()), (seq + 20) as u8);
        log_frame("GCS ->", &item_frame);
        if let Some(parsed) = round_trip_message(&dialect, item) {
            if let Some(parsed_item) = downcast_message::<MissionItem>(parsed.as_ref()) {
                println!(
                    "  uploaded seq={} cmd={:?}",
                    parsed_item.seq, parsed_item.command
                );
            }
        }
    }

    let ack = MissionAck {
        target_system: GCS_SYSTEM_ID,
        target_component: GCS_COMPONENT_ID,
        r#type: MavMissionResult::MAV_MISSION_ACCEPTED,
        mission_type,
    };
    let ack_frame = frame_from_drone(Box::new(ack.clone()), 99);
    log_frame("Drone ->", &ack_frame);
    if let Some(parsed) = round_trip_message(&dialect, &ack) {
        if let Some(parsed_ack) = downcast_message::<MissionAck>(parsed.as_ref()) {
            println!("Mission upload complete: {:?}", parsed_ack.r#type);
        }
    }
}
