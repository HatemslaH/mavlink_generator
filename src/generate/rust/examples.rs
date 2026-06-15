use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::camel_case;

pub struct RustExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/rust/examples/README.md"),
    ),
    (
        "common.rs",
        include_str!("../../../templates/rust/examples/common.rs"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

impl LanguageExampleGenerator for RustExampleGenerator {
    fn static_files(&self) -> Vec<ExampleFile> {
        STATIC_TEMPLATES
            .iter()
            .map(|(name, content)| ExampleFile {
                relative_path: PathBuf::from(*name),
                content: (*content).to_string(),
            })
            .collect()
    }

    fn generated_files(&self, dialect_stems: &[String]) -> Vec<ExampleFile> {
        dialect_stems
            .iter()
            .flat_map(|stem| {
                let stem = stem.clone();
                GENERATED_EXAMPLES
                    .iter()
                    .map(move |(suffix, render)| ExampleFile {
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.rs")),
                        content: render(&stem),
                    })
            })
            .collect()
    }
}

fn dialect_struct_name(stem: &str) -> String {
    format!("MavlinkDialect{}", camel_case(stem))
}

fn render_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Example for the `{dialect_stem}` dialect: serialize a Heartbeat frame and parse it back.

mod common;

use common::*;

fn main() {{
    let dialect = {dialect_struct};

    let heartbeat = Heartbeat {{
        custom_mode: 0,
        r#type: MavType::MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot::MAV_AUTOPILOT_PX4,
        base_mode: 0,
        system_status: MavState::MAV_STATE_ACTIVE,
        mavlink_version: dialect.version(),
    }};

    let frame = frame_from_gcs(Box::new(heartbeat.clone()), 0);
    let wire = frame.serialize();
    log_frame("GCS ->", &frame);
    println!("Serialized HEARTBEAT ({{}} bytes)", wire.len());

    if let Some(parsed) = round_trip_message(&dialect, &heartbeat) {{
        if let Some(heartbeat) = downcast_message::<Heartbeat>(parsed.as_ref()) {{
            println!(
                "Parsed HEARTBEAT type={{:?}} status={{:?}}",
                heartbeat.r#type, heartbeat.system_status
            );
        }}
    }}
}}
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Virtual mission upload for the `{dialect_stem}` dialect.

mod common;

use common::*;

fn main() {{
    let dialect = {dialect_struct};
    let mission_type = MavMissionType::MAV_MISSION_TYPE_MISSION;

    let mission_items = [
        MissionItem {{
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
        }},
        MissionItem {{
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
        }},
    ];

    let count = MissionCount {{
        count: mission_items.len() as u16,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        mission_type,
    }};
    let count_frame = frame_from_gcs(Box::new(count.clone()), 1);
    log_frame("GCS ->", &count_frame);
    let _ = round_trip_message(&dialect, &count);

    for (seq, item) in mission_items.iter().enumerate() {{
        let request = MissionRequest {{
            seq: seq as u16,
            target_system: GCS_SYSTEM_ID,
            target_component: GCS_COMPONENT_ID,
            mission_type,
        }};
        let request_frame = frame_from_drone(Box::new(request.clone()), (seq + 10) as u8);
        log_frame("Drone ->", &request_frame);
        let _ = round_trip_message(&dialect, &request);

        let item_frame = frame_from_gcs(Box::new(item.clone()), (seq + 20) as u8);
        log_frame("GCS ->", &item_frame);
        if let Some(parsed) = round_trip_message(&dialect, item) {{
            if let Some(parsed_item) = downcast_message::<MissionItem>(parsed.as_ref()) {{
                println!(
                    "  uploaded seq={{}} cmd={{:?}}",
                    parsed_item.seq, parsed_item.command
                );
            }}
        }}
    }}

    let ack = MissionAck {{
        target_system: GCS_SYSTEM_ID,
        target_component: GCS_COMPONENT_ID,
        r#type: MavMissionResult::MAV_MISSION_ACCEPTED,
        mission_type,
    }};
    let ack_frame = frame_from_drone(Box::new(ack.clone()), 99);
    log_frame("Drone ->", &ack_frame);
    if let Some(parsed) = round_trip_message(&dialect, &ack) {{
        if let Some(parsed_ack) = downcast_message::<MissionAck>(parsed.as_ref()) {{
            println!("Mission upload complete: {{:?}}", parsed_ack.r#type);
        }}
    }}
}}
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Virtual telemetry request for the `{dialect_stem}` dialect.

mod common;

use common::*;

fn main() {{
    let dialect = {dialect_struct};

    let set_interval = CommandLong {{
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
    }};
    let interval_frame = frame_from_gcs(Box::new(set_interval.clone()), 1);
    log_frame("GCS ->", &interval_frame);
    if let Some(parsed) = round_trip_message(&dialect, &set_interval) {{
        if let Some(parsed_interval) = downcast_message::<CommandLong>(parsed.as_ref()) {{
            println!(
                "  SET_MESSAGE_INTERVAL msgId={{}} interval_us={{}}",
                parsed_interval.param1 as u32, parsed_interval.param2 as u32
            );
        }}
    }}

    let request_once = CommandLong {{
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
    }};
    let once_frame = frame_from_gcs(Box::new(request_once.clone()), 2);
    log_frame("GCS ->", &once_frame);
    let _ = round_trip_message(&dialect, &request_once);

    let attitude = Attitude {{
        time_boot_ms: 12_345,
        roll: 0.01,
        pitch: -0.02,
        yaw: 1.57,
        rollspeed: 0.0,
        pitchspeed: 0.0,
        yawspeed: 0.0,
    }};
    let telemetry_frame = frame_from_drone(Box::new(attitude.clone()), 3);
    log_frame("Drone ->", &telemetry_frame);
    if let Some(parsed) = round_trip_message(&dialect, &attitude) {{
        if let Some(parsed_attitude) = downcast_message::<Attitude>(parsed.as_ref()) {{
            println!(
                "  ATTITUDE roll={{}} pitch={{}} yaw={{}}",
                parsed_attitude.roll, parsed_attitude.pitch, parsed_attitude.yaw
            );
        }}
    }}
}}
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Virtual parameter service for the `{dialect_stem}` dialect.

mod common;

use common::*;

struct SimulatedParam {{
    id: &'static str,
    value: f32,
    index: u16,
}}

fn main() {{
    let dialect = {dialect_struct};

    let list_request = ParamRequestList {{
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
    }};
    let list_frame = frame_from_gcs(Box::new(list_request.clone()), 1);
    log_frame("GCS ->", &list_frame);
    let _ = round_trip_message(&dialect, &list_request);

    let simulated_params = [
        SimulatedParam {{
            id: "SYSID_THISMAV",
            value: 1.0,
            index: 0,
        }},
        SimulatedParam {{
            id: "SYSID_MYGCS",
            value: 255.0,
            index: 1,
        }},
        SimulatedParam {{
            id: "COMPASS_ENABLE",
            value: 1.0,
            index: 2,
        }},
    ];

    for param in &simulated_params {{
        let value = ParamValue {{
            param_value: param.value,
            param_count: simulated_params.len() as u16,
            param_index: param.index,
            param_id: param_id_from_string(param.id),
            param_type: MavParamType::MAV_PARAM_TYPE_REAL32,
        }};
        let value_frame = frame_from_drone(Box::new(value.clone()), param.index as u8 + 10);
        log_frame("Drone ->", &value_frame);
        if let Some(parsed) = round_trip_message(&dialect, &value) {{
            if let Some(parsed_value) = downcast_message::<ParamValue>(parsed.as_ref()) {{
                println!(
                    "  PARAM_VALUE [{{}}/{{}}] {{}}={{}}",
                    param.index + 1,
                    simulated_params.len(),
                    param_id_to_string(&parsed_value.param_id),
                    parsed_value.param_value
                );
            }}
        }}
    }}

    let param_name = "SYSID_THISMAV";
    let read_request = ParamRequestRead {{
        param_index: -1,
        target_system: DRONE_SYSTEM_ID,
        target_component: DRONE_COMPONENT_ID,
        param_id: param_id_from_string(param_name),
    }};
    let read_frame = frame_from_gcs(Box::new(read_request.clone()), 50);
    log_frame("GCS ->", &read_frame);
    if let Some(parsed) = round_trip_message(&dialect, &read_request) {{
        if let Some(parsed_read) = downcast_message::<ParamRequestRead>(parsed.as_ref()) {{
            println!(
                "  PARAM_REQUEST_READ id={{}}",
                param_id_to_string(&parsed_read.param_id)
            );
        }}
    }}

    let single_value = ParamValue {{
        param_value: 1.0,
        param_count: simulated_params.len() as u16,
        param_index: 0,
        param_id: param_id_from_string(param_name),
        param_type: MavParamType::MAV_PARAM_TYPE_REAL32,
    }};
    let single_frame = frame_from_drone(Box::new(single_value.clone()), 51);
    log_frame("Drone ->", &single_frame);
    let _ = round_trip_message(&dialect, &single_value);
}}
"#
    )
}
