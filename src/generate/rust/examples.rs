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
    (
        "protocols_common.rs",
        include_str!("../../../templates/rust/examples/protocols_common.rs"),
    ),
];

const LOW_LEVEL_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

const PROTOCOL_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("protocol_mission", render_protocol_mission_example),
    ("protocol_parameters", render_protocol_parameters_example),
    ("protocol_command", render_protocol_command_example),
    ("protocol_heartbeat", render_protocol_heartbeat_example),
    ("protocol_vehicle", render_protocol_vehicle_example),
    ("protocol_subscribe", render_protocol_subscribe_example),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::examples::{
        ALL_EXAMPLE_SUFFIXES, LOW_LEVEL_EXAMPLE_SUFFIXES, PROTOCOL_EXAMPLE_SUFFIXES,
    };

    #[test]
    fn example_suffixes_match_shared_constants() {
        let low_level: Vec<_> = LOW_LEVEL_EXAMPLES.iter().map(|(s, _)| *s).collect();
        let protocol: Vec<_> = PROTOCOL_EXAMPLES.iter().map(|(s, _)| *s).collect();
        let all: Vec<_> = LOW_LEVEL_EXAMPLES
            .iter()
            .chain(PROTOCOL_EXAMPLES.iter())
            .map(|(s, _)| *s)
            .collect();

        assert_eq!(low_level, LOW_LEVEL_EXAMPLE_SUFFIXES);
        assert_eq!(protocol, PROTOCOL_EXAMPLE_SUFFIXES);
        assert_eq!(all, ALL_EXAMPLE_SUFFIXES);
    }
}

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
                LOW_LEVEL_EXAMPLES
                    .iter()
                    .chain(PROTOCOL_EXAMPLES.iter())
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

fn render_protocol_mission_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Mission protocol example for the `{dialect_stem}` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{{CommandProtocol, CommandServer, MissionItems, MissionProtocol, MissionServer}};

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let link = create_virtual_link(dialect);

    let mission_server = MissionServer::new(
        Arc::clone(&link.drone),
        None,
        mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
    );
    let _command_server = CommandServer::new(Arc::clone(&link.drone), None, None);
    let mission_protocol = MissionProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(3),
        Duration::from_secs(10),
    );

    let plan = vec![
        MissionItems::waypoint(
            0,
            47.397_742,
            8.545_594,
            50.0,
            DRONE_SYSTEM_ID,
            DRONE_COMPONENT_ID,
            mavlink::MavCmd::MAV_CMD_NAV_WAYPOINT,
            mavlink::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
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
            DRONE_SYSTEM_ID,
            DRONE_COMPONENT_ID,
            mavlink::MavCmd::MAV_CMD_NAV_WAYPOINT,
            mavlink::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
    ];

    let upload_result = mission_protocol
        .upload(
            plan,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&|sent, total, item| {{
                println!(
                    "Upload progress {{}}/{{}} seq={{}} cmd={{:?}}",
                    sent, total, item.seq, item.command
                );
            }}),
            None,
        )
        .await
        .expect("upload should succeed");
    println!("Mission upload result: {{upload_result:?}}");
    println!("Vehicle stored {{}} items", mission_server.items().len());

    let downloaded = mission_protocol
        .download(
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&|received, total, item| {{
                println!(
                    "Download progress {{}}/{{}} seq={{}}",
                    received, total, item.seq
                );
            }}),
            None,
        )
        .await
        .expect("download should succeed");
    println!("Downloaded {{}} mission items", downloaded.len());

    let command_protocol = CommandProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(5),
    );
    let set_current = mission_protocol
        .set_current_with_command(0, Some(&command_protocol), true, false, None)
        .await
        .expect("set current should succeed");
    println!(
        "Set current seq={{}} ack={{:?}}",
        set_current.sequence,
        set_current.command_ack.map(|ack| ack.result)
    );

    let clear_result = mission_protocol
        .clear(mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION, None)
        .await
        .expect("clear should succeed");
    println!("Mission clear result: {{clear_result:?}}");

    mission_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}}
"#
    )
}

fn render_protocol_parameters_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Parameter protocol example for the `{dialect_stem}` dialect.

mod protocols_common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{{ParameterProtocol, ParameterServer}};

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let link = create_virtual_link(dialect);

    let mut initial = HashMap::new();
    initial.insert(
        "SYSID_THISMAV".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    initial.insert(
        "SYSID_MYGCS".to_string(),
        (255.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    initial.insert(
        "COMPASS_ENABLE".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );

    let parameter_server = ParameterServer::from_typed(Arc::clone(&link.drone), initial);
    let parameter_protocol = ParameterProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_millis(500),
        Duration::from_secs(3),
    );

    let all_params = parameter_protocol
        .fetch_all(
            Some(&|entry, received, expected| {{
                println!("  [{{}}/{{}}] {{}}={{}}", received, expected, entry.id, entry.value);
            }}),
            None,
        )
        .await
        .expect("fetch all should succeed");
    println!(
        "Fetched {{}} parameters (cache size={{}})",
        all_params.len(),
        parameter_protocol.cache().len()
    );

    let single = parameter_protocol
        .read_by_name("SYSID_THISMAV", None)
        .await
        .expect("read should succeed");
    println!("Read SYSID_THISMAV={{}}", single.value);

    let updated = parameter_protocol
        .write_by_name("COMPASS_ENABLE", 0.0, None, None)
        .await
        .expect("write should succeed");
    println!("Wrote COMPASS_ENABLE={{}} ({{:?}})", updated.value, updated.param_type);

    parameter_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}}
"#
    )
}

fn render_protocol_command_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Command protocol example for the `{dialect_stem}` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{{CommandProtocol, CommandServer}};

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let link = create_virtual_link(dialect);

    let command_server = CommandServer::new(Arc::clone(&link.drone), None, None);
    let command_protocol = CommandProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(5),
    );

    let interval_ack = command_protocol
        .set_message_interval(mavlink::Attitude::MSG_ID, 100_000, None, None)
        .await
        .expect("set interval should succeed");
    println!("SET_MESSAGE_INTERVAL ack: {{:?}}", interval_ack.result);

    let request_ack = command_protocol
        .request_message(mavlink::Attitude::MSG_ID, 0.0, None, None)
        .await
        .expect("request message should succeed");
    println!("REQUEST_MESSAGE ack: {{:?}}", request_ack.result);

    let arm_ack = command_protocol
        .arm(false, None, None)
        .await
        .expect("arm should succeed");
    println!("ARM ack: {{:?}}", arm_ack.result);

    let disarm_ack = command_protocol
        .disarm(false, None, None)
        .await
        .expect("disarm should succeed");
    println!("DISARM ack: {{:?}}", disarm_ack.result);

    command_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}}
"#
    )
}

fn render_protocol_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Heartbeat protocol example for the `{dialect_stem}` dialect.

mod protocols_common;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{{HeartbeatMonitor, HeartbeatPublisher, HeartbeatTemplates}};

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let link = create_virtual_link(dialect);

    let gcs_publisher = HeartbeatPublisher::new(
        Arc::clone(&link.gcs),
        HeartbeatTemplates::gcs(dialect.version()),
        Duration::from_millis(500),
    );
    let drone_publisher = HeartbeatPublisher::new(
        Arc::clone(&link.drone),
        HeartbeatTemplates::autopilot_default(dialect.version()),
        Duration::from_millis(500),
    );
    let gcs_monitor = HeartbeatMonitor::new(
        Arc::clone(&link.gcs),
        Duration::from_secs(2),
        None,
        None,
    );

    gcs_monitor.start();
    gcs_publisher.start();
    drone_publisher.start();

    let mut exclude = HashSet::new();
    exclude.insert(GCS_SYSTEM_ID);
    let vehicle = gcs_monitor
        .wait_for_vehicle(Some(exclude), Duration::from_secs(5), None)
        .await
        .expect("vehicle should be discovered");
    println!("Vehicle discovered: {{vehicle}}");
    println!("Drone online: {{}}", gcs_monitor.is_online(vehicle));
    if let Some(state) = gcs_monitor.state_for(vehicle) {{
        println!(
            "Drone heartbeat: type={{:?}} status={{:?}}",
            state.heartbeat.r#type, state.heartbeat.system_status
        );
    }}

    drone_publisher.stop();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    println!("Drone online after stop: {{}}", gcs_monitor.is_online(vehicle));

    gcs_monitor.stop().await;
    gcs_publisher.stop();
    close_virtual_link(link).await.expect("close should succeed");
}}
"#
    )
}

fn render_protocol_vehicle_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! MavlinkGcs / MavlinkVehicleClient facade example for `{dialect_stem}`.

mod protocols_common;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{{
    CommandServer, HeartbeatPublisher, HeartbeatTemplates, MavlinkGcs, MavlinkSession,
    ParameterServer, VirtualMavlinkBus,
}};

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let bus = VirtualMavlinkBus::new();
    let gcs_link = bus.create_endpoint();
    let drone_link = bus.create_endpoint();

    let gcs = MavlinkGcs::connect(
        dialect.clone(),
        gcs_link,
        GCS_SYSTEM_ID,
        GCS_COMPONENT_ID,
        Duration::from_millis(500),
        Duration::from_secs(3),
    );

    let drone_session = Arc::new(MavlinkSession::new(
        dialect.clone(),
        drone_link,
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        mavlink::MavlinkVersion::V2,
    ));

    let drone_publisher = HeartbeatPublisher::new(
        Arc::clone(&drone_session),
        HeartbeatTemplates::autopilot_default(dialect.version()),
        Duration::from_millis(500),
    );

    let mut initial = HashMap::new();
    initial.insert(
        "SYSID_THISMAV".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    let parameter_server = ParameterServer::from_typed(Arc::clone(&drone_session), initial);
    let command_server = CommandServer::new(Arc::clone(&drone_session), None, None);

    gcs.start();
    drone_publisher.start();

    let mut exclude = HashSet::new();
    exclude.insert(GCS_SYSTEM_ID);
    let client = gcs
        .wait_for_vehicle(Some(exclude), Duration::from_secs(5))
        .await
        .expect("vehicle should connect");
    println!("Connected to vehicle {{}}", client.vehicle);

    let params = client
        .parameters
        .fetch_all(None, None)
        .await
        .expect("fetch all should succeed");
    println!("Vehicle has {{}} parameters", params.len());

    let ack = client
        .command
        .request_message(mavlink::Heartbeat::MSG_ID, 0.0, None, None)
        .await
        .expect("request message should succeed");
    println!("REQUEST_MESSAGE ack: {{:?}}", ack.result);

    parameter_server.close().await;
    command_server.close().await;
    drone_publisher.stop();
    drone_session.close().await.expect("close drone session");
    gcs.close().await.expect("close gcs");
    bus.close_all().await;
}}
"#
    )
}

fn render_protocol_subscribe_example(dialect_stem: &str) -> String {
    let dialect_struct = dialect_struct_name(dialect_stem);

    format!(
        r#"//! Typed message subscription example for the `{dialect_stem}` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::MavlinkNode;

#[tokio::main]
async fn main() {{
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new({dialect_struct});
    let link = create_virtual_link(dialect);
    let vehicle = MavlinkNode::new(DRONE_SYSTEM_ID, DRONE_COMPONENT_ID);

    let mut attitude_samples = Vec::new();
    let mut subscription = link.gcs.listen_message::<mavlink::Attitude, _>(
        |message, _frame| attitude_samples.push((*message).clone()),
        Some(vehicle.system_id),
        None,
    );

    link.drone
        .send(Box::new(mavlink::Attitude {{
            time_boot_ms: 1000,
            roll: 0.1,
            pitch: -0.05,
            yaw: 1.57,
            rollspeed: 0.0,
            pitchspeed: 0.0,
            yawspeed: 0.0,
        }}))
        .await
        .expect("send should succeed");

    tokio::time::sleep(Duration::from_millis(50)).await;
    subscription.cancel();

    println!(
        "Received {{}} ATTITUDE samples via listen_message",
        attitude_samples.len()
    );
    if let Some(sample) = attitude_samples.first() {{
        println!(
            "  roll={{}} pitch={{}} yaw={{}}",
            sample.roll, sample.pitch, sample.yaw
        );
    }}

    close_virtual_link(link).await.expect("close should succeed");
}}
"#
    )
}
