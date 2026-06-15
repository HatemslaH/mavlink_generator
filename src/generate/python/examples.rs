use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct PythonExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/py/examples/README.md"),
    ),
    (
        "common.py",
        include_str!("../../../templates/py/examples/common.py"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

impl LanguageExampleGenerator for PythonExampleGenerator {
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
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.py")),
                        content: render(&stem),
                    })
            })
            .collect()
    }
}

fn dialect_class_name(stem: &str) -> String {
    format!("MavlinkDialect{}", capitalize(stem))
}

fn render_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env python3
"""Example for the `{dialect_stem}` dialect: serialize a Heartbeat frame and parse it back."""

from common import *


def main() -> None:
    dialect = {dialect_class}()

    heartbeat = Heartbeat(
        custom_mode=0,
        type=MavType.MAV_TYPE_QUADROTOR,
        autopilot=MavAutopilot.MAV_AUTOPILOT_PX4,
        base_mode=0,
        system_status=MavState.MAV_STATE_ACTIVE,
        mavlink_version=dialect.version,
    )

    frame = frame_from_gcs(heartbeat)
    wire = frame.serialize()
    log_frame("GCS ->", frame)
    print(f"Serialized HEARTBEAT ({{len(wire)}} bytes)")

    parsed = round_trip_message(dialect, heartbeat)
    if isinstance(parsed, Heartbeat):
        print(f"Parsed HEARTBEAT type={{parsed.type}} status={{parsed.system_status}}")


if __name__ == "__main__":
    main()
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env python3
"""Virtual mission upload for the `{dialect_stem}` dialect.

Follows https://mavlink.io/en/services/mission.html upload sequence:
GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
"""

from common import *


def main() -> None:
    dialect = {dialect_class}()
    mission_type = MavMissionType.MAV_MISSION_TYPE_MISSION

    mission_items = [
        MissionItem(
            param1=0,
            param2=2,
            param3=0,
            param4=0,
            x=47.397742,
            y=8.545594,
            z=50,
            seq=0,
            command=MavCmd.MAV_CMD_NAV_WAYPOINT,
            target_system=drone_system_id,
            target_component=drone_component_id,
            frame=MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current=0,
            autocontinue=1,
            mission_type=mission_type,
        ),
        MissionItem(
            param1=0,
            param2=2,
            param3=0,
            param4=0,
            x=47.398000,
            y=8.546000,
            z=50,
            seq=1,
            command=MavCmd.MAV_CMD_NAV_WAYPOINT,
            target_system=drone_system_id,
            target_component=drone_component_id,
            frame=MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current=0,
            autocontinue=1,
            mission_type=mission_type,
        ),
    ]

    seq = 0

    # 1. GCS announces mission size.
    count = MissionCount(
        count=len(mission_items),
        target_system=drone_system_id,
        target_component=drone_component_id,
        mission_type=mission_type,
    )
    count_frame = frame_from_gcs(count, sequence=1)
    log_frame("GCS ->", count_frame)
    round_trip_message(dialect, count)

    # 2. Drone requests each mission item, GCS responds.
    while seq < len(mission_items):
        request = MissionRequest(
            seq=seq,
            target_system=gcs_system_id,
            target_component=gcs_component_id,
            mission_type=mission_type,
        )
        request_frame = frame_from_drone(request, sequence=seq + 10)
        log_frame("Drone ->", request_frame)
        round_trip_message(dialect, request)

        item = mission_items[seq]
        item_frame = frame_from_gcs(item, sequence=seq + 20)
        log_frame("GCS ->", item_frame)
        parsed_item = round_trip_message(dialect, item)
        if isinstance(parsed_item, MissionItem):
            print(f"  uploaded seq={{parsed_item.seq}} cmd={{parsed_item.command}}")

        seq += 1

    # 3. Drone accepts the mission.
    ack = MissionAck(
        target_system=gcs_system_id,
        target_component=gcs_component_id,
        type=MavMissionResult.MAV_MISSION_ACCEPTED,
        mission_type=mission_type,
    )
    ack_frame = frame_from_drone(ack, sequence=99)
    log_frame("Drone ->", ack_frame)
    parsed_ack = round_trip_message(dialect, ack)
    if isinstance(parsed_ack, MissionAck):
        print(f"Mission upload complete: {{parsed_ack.type}}")


if __name__ == "__main__":
    main()
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env python3
"""Virtual telemetry request for the `{dialect_stem}` dialect.

Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
"""

from common import *


def main() -> None:
    dialect = {dialect_class}()

    # Stream ATTITUDE (msg id 30) at 10 Hz (100_000 microseconds).
    set_interval = CommandLong(
        param1=float(Attitude.MSG_ID),
        param2=100000,
        param3=0,
        param4=0,
        param5=0,
        param6=0,
        param7=0,
        command=MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
        target_system=drone_system_id,
        target_component=drone_component_id,
        confirmation=0,
    )
    interval_frame = frame_from_gcs(set_interval, sequence=1)
    log_frame("GCS ->", interval_frame)
    parsed_interval = round_trip_message(dialect, set_interval)
    if isinstance(parsed_interval, CommandLong):
        print(
            f"  SET_MESSAGE_INTERVAL msgId={{int(parsed_interval.param1)}} "
            f"interval_us={{int(parsed_interval.param2)}}"
        )

    # One-shot ATTITUDE sample via MAV_CMD_REQUEST_MESSAGE.
    request_once = CommandLong(
        param1=float(Attitude.MSG_ID),
        param2=0,
        param3=0,
        param4=0,
        param5=0,
        param6=0,
        param7=0,
        command=MavCmd.MAV_CMD_REQUEST_MESSAGE,
        target_system=drone_system_id,
        target_component=drone_component_id,
        confirmation=0,
    )
    once_frame = frame_from_gcs(request_once, sequence=2)
    log_frame("GCS ->", once_frame)
    round_trip_message(dialect, request_once)

    # Simulated vehicle response: ATTITUDE telemetry frame.
    attitude = Attitude(
        time_boot_ms=12345,
        roll=0.01,
        pitch=-0.02,
        yaw=1.57,
        rollspeed=0,
        pitchspeed=0,
        yawspeed=0,
    )
    telemetry_frame = frame_from_drone(attitude, sequence=3)
    log_frame("Drone ->", telemetry_frame)
    parsed_attitude = round_trip_message(dialect, attitude)
    if isinstance(parsed_attitude, Attitude):
        print(
            f"  ATTITUDE roll={{parsed_attitude.roll}} "
            f"pitch={{parsed_attitude.pitch}} yaw={{parsed_attitude.yaw}}"
        )


if __name__ == "__main__":
    main()
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env python3
"""Virtual parameter service for the `{dialect_stem}` dialect.

Follows https://mavlink.io/en/services/parameter.html:
PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
"""

from dataclasses import dataclass

from common import *


@dataclass
class SimulatedParam:
    id: str
    value: float
    index: int


def main() -> None:
    dialect = {dialect_class}()

    # 1. GCS requests the full onboard parameter set.
    list_request = ParamRequestList(
        target_system=drone_system_id,
        target_component=drone_component_id,
    )
    list_frame = frame_from_gcs(list_request, sequence=1)
    log_frame("GCS ->", list_frame)
    round_trip_message(dialect, list_request)

    # 2. Drone responds with PARAM_VALUE messages (simulated subset).
    simulated_params = [
        SimulatedParam("SYSID_THISMAV", 1, 0),
        SimulatedParam("SYSID_MYGCS", 255, 1),
        SimulatedParam("COMPASS_ENABLE", 1, 2),
    ]

    for param in simulated_params:
        value = ParamValue(
            param_value=param.value,
            param_count=len(simulated_params),
            param_index=param.index,
            param_id=param_id_from_string(param.id),
            param_type=MavParamType.MAV_PARAM_TYPE_REAL32,
        )
        value_frame = frame_from_drone(value, sequence=param.index + 10)
        log_frame("Drone ->", value_frame)
        parsed = round_trip_message(dialect, value)
        if isinstance(parsed, ParamValue):
            print(
                f"  PARAM_VALUE [{{param.index + 1}}/{{len(simulated_params)}}] "
                f"{{param_id_to_string(parsed.param_id)}}={{parsed.param_value}}"
            )

    # 3. GCS requests one parameter by name (param_index = -1).
    param_name = "SYSID_THISMAV"
    read_request = ParamRequestRead(
        param_index=-1,
        target_system=drone_system_id,
        target_component=drone_component_id,
        param_id=param_id_from_string(param_name),
    )
    read_frame = frame_from_gcs(read_request, sequence=50)
    log_frame("GCS ->", read_frame)
    parsed_read = round_trip_message(dialect, read_request)
    if isinstance(parsed_read, ParamRequestRead):
        print(f"  PARAM_REQUEST_READ id={{param_id_to_string(parsed_read.param_id)}}")

    # 4. Drone answers with the matching PARAM_VALUE.
    single_value = ParamValue(
        param_value=1,
        param_count=len(simulated_params),
        param_index=0,
        param_id=param_id_from_string(param_name),
        param_type=MavParamType.MAV_PARAM_TYPE_REAL32,
    )
    single_frame = frame_from_drone(single_value, sequence=51)
    log_frame("Drone ->", single_frame)
    round_trip_message(dialect, single_value)


if __name__ == "__main__":
    main()
"#
    )
}
