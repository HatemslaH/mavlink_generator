//! Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).

use mavlink::{protocols::MissionItems, MavCmd, MavFrame, MavMissionType, MissionItemInt};

/// Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).
pub fn build_sample_mission(target_system: u8, target_component: u8) -> Vec<MissionItemInt> {
    MissionItems::with_sequential_seq(vec![
        MissionItems::waypoint(
            0,
            47.397_742,
            8.545_594,
            50.0,
            target_system,
            target_component,
            MavCmd::MAV_CMD_NAV_WAYPOINT,
            MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
        MissionItems::waypoint(
            1,
            47.398_000,
            8.546_000,
            50.0,
            target_system,
            target_component,
            MavCmd::MAV_CMD_NAV_WAYPOINT,
            MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
        MissionItems::waypoint(
            2,
            47.398_258,
            8.546_406,
            50.0,
            target_system,
            target_component,
            MavCmd::MAV_CMD_NAV_RETURN_TO_LAUNCH,
            MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
    ])
}

pub fn describe_mission_item(item: &MissionItemInt) -> String {
    let lat = f64::from(item.x) / 1e7;
    let lon = f64::from(item.y) / 1e7;
    format!(
        "seq={} {:?} lat={lat:.6} lon={lon:.6} alt={}m",
        item.seq, item.command, item.z
    )
}
