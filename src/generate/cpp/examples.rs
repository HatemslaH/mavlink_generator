use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct CppExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/cpp/examples/README.md"),
    ),
    (
        "common.hpp",
        include_str!("../../../templates/cpp/examples/common.hpp"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

impl LanguageExampleGenerator for CppExampleGenerator {
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
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.cpp")),
                        content: render(&stem),
                    })
            })
            .collect()
    }
}

fn dialect_var_name(stem: &str) -> String {
    format!("mavlink_dialect_{}", stem.to_lowercase())
}

fn dialect_type_name(stem: &str) -> String {
    format!("mavlink_dialect_{}_t", capitalize(stem).to_lowercase())
}

fn render_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_var = dialect_var_name(dialect_stem);
    let dialect_type = dialect_type_name(dialect_stem);

    format!(
        r#"#include <cstdio>

#include "common.hpp"

/// Example for the `{dialect_stem}` dialect: serialize a HEARTBEAT frame and
/// parse it back with [{dialect_type}].
int main() {{
  mavlink::{dialect_type} dialect;
  mavlink::{dialect_var}_init(dialect);

  mavlink::heartbeat_t heartbeat{{}};
  heartbeat.custom_mode = 0;
  heartbeat.type = mavlink::MAV_TYPE_QUADROTOR;
  heartbeat.autopilot = mavlink::MAV_AUTOPILOT_PX4;
  heartbeat.base_mode = 0;
  heartbeat.system_status = mavlink::MAV_STATE_ACTIVE;
  heartbeat.mavlink_version = dialect.base.version;

  uint8_t payload[mavlink::heartbeat_ENCODED_LENGTH];
  mavlink::heartbeat_serialize(heartbeat, payload);

  mavlink::frame_t frame;
  mavlink_frame_from_gcs(
    frame,
    0,
    mavlink::heartbeat_MSG_ID,
    mavlink::heartbeat_CRC_EXTRA,
    payload,
    mavlink::heartbeat_ENCODED_LENGTH
  );

  uint8_t wire[mavlink::MAVLINK_MAX_FRAME_SIZE];
  size_t wire_len = mavlink::mavlink_frame_serialize_v2(frame, wire, sizeof(wire));
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  std::printf("Serialized HEARTBEAT (%zu bytes)\n", wire_len);

  mavlink::heartbeat_t parsed{{}};
  dialect.base.parse(
    &dialect.base,
    mavlink::heartbeat_MSG_ID,
    payload,
    mavlink::heartbeat_ENCODED_LENGTH,
    &parsed
  );
  std::printf("Parsed HEARTBEAT type=%d status=%d\n", static_cast<int>(parsed.type), static_cast<int>(parsed.system_status));

  return 0;
}}
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_var = dialect_var_name(dialect_stem);
    let dialect_type = dialect_type_name(dialect_stem);

    format!(
        r#"#include <cstdio>

#include "common.hpp"

/// Virtual mission upload for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
int main() {{
  mavlink::{dialect_type} dialect;
  mavlink::{dialect_var}_init(dialect);

  const auto mission_type = mavlink::MAV_MISSION_TYPE_MISSION;

  mavlink::mission_item_t mission_items[2] = {{
    {{
      0, 2, 0, 0,
      47.397742f, 8.545594f, 50,
      0,
      mavlink::MAV_CMD_NAV_WAYPOINT,
      DRONE_SYSTEM_ID,
      DRONE_COMPONENT_ID,
      mavlink::MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1,
      mission_type,
    }},
    {{
      0, 2, 0, 0,
      47.398000f, 8.546000f, 50,
      1,
      mavlink::MAV_CMD_NAV_WAYPOINT,
      DRONE_SYSTEM_ID,
      DRONE_COMPONENT_ID,
      mavlink::MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1,
      mission_type,
    }},
  }};

  uint8_t payload[255];
  mavlink::frame_t frame;

  mavlink::mission_count_t count{{}};
  count.count = 2;
  count.target_system = DRONE_SYSTEM_ID;
  count.target_component = DRONE_COMPONENT_ID;
  count.mission_type = mission_type;
  mavlink::mission_count_serialize(count, payload);
  mavlink_frame_from_gcs(
    frame,
    1,
    mavlink::mission_count_MSG_ID,
    mavlink::mission_count_CRC_EXTRA,
    payload,
    mavlink::mission_count_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  mavlink::mission_count_parse(payload, count);

  for (uint16_t seq = 0; seq < 2; seq++) {{
    mavlink::mission_request_t request{{}};
    request.seq = seq;
    request.target_system = GCS_SYSTEM_ID;
    request.target_component = GCS_COMPONENT_ID;
    request.mission_type = mission_type;
    mavlink::mission_request_serialize(request, payload);
    mavlink_frame_from_drone(
      frame,
      static_cast<uint8_t>(seq + 10),
      mavlink::mission_request_MSG_ID,
      mavlink::mission_request_CRC_EXTRA,
      payload,
      mavlink::mission_request_ENCODED_LENGTH
    );
    mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);
    mavlink::mission_request_parse(payload, request);

    mavlink::mission_item_t item = mission_items[seq];
    mavlink::mission_item_serialize(item, payload);
    mavlink_frame_from_gcs(
      frame,
      static_cast<uint8_t>(seq + 20),
      mavlink::mission_item_MSG_ID,
      mavlink::mission_item_CRC_EXTRA,
      payload,
      mavlink::mission_item_ENCODED_LENGTH
    );
    mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

    mavlink::mission_item_t parsed_item{{}};
    mavlink::mission_item_parse(payload, parsed_item);
    std::printf(
      "  uploaded seq=%u cmd=%u\n",
      parsed_item.seq,
      static_cast<unsigned>(parsed_item.command)
    );
  }}

  mavlink::mission_ack_t ack{{}};
  ack.target_system = GCS_SYSTEM_ID;
  ack.target_component = GCS_COMPONENT_ID;
  ack.type = mavlink::MAV_MISSION_ACCEPTED;
  ack.mission_type = mission_type;
  mavlink::mission_ack_serialize(ack, payload);
  mavlink_frame_from_drone(
    frame,
    99,
    mavlink::mission_ack_MSG_ID,
    mavlink::mission_ack_CRC_EXTRA,
    payload,
    mavlink::mission_ack_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

  mavlink::mission_ack_t parsed_ack{{}};
  mavlink::mission_ack_parse(payload, parsed_ack);
  std::printf("Mission upload complete: %d\n", static_cast<int>(parsed_ack.type));

  (void)dialect;
  return 0;
}}
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_var = dialect_var_name(dialect_stem);
    let dialect_type = dialect_type_name(dialect_stem);

    format!(
        r#"#include <cstdio>

#include "common.hpp"

/// Virtual telemetry request for the `{dialect_stem}` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
int main() {{
  mavlink::{dialect_type} dialect;
  mavlink::{dialect_var}_init(dialect);

  uint8_t payload[mavlink::command_long_ENCODED_LENGTH];
  mavlink::frame_t frame;

  mavlink::command_long_t set_interval{{}};
  set_interval.param1 = mavlink::attitude_MSG_ID;
  set_interval.param2 = 100000;
  set_interval.command = mavlink::MAV_CMD_SET_MESSAGE_INTERVAL;
  set_interval.target_system = DRONE_SYSTEM_ID;
  set_interval.target_component = DRONE_COMPONENT_ID;
  mavlink::command_long_serialize(set_interval, payload);
  mavlink_frame_from_gcs(
    frame,
    1,
    mavlink::command_long_MSG_ID,
    mavlink::command_long_CRC_EXTRA,
    payload,
    mavlink::command_long_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

  mavlink::command_long_t parsed_interval{{}};
  mavlink::command_long_parse(payload, parsed_interval);
  std::printf(
    "  SET_MESSAGE_INTERVAL msgId=%.0f interval_us=%.0f\n",
    parsed_interval.param1,
    parsed_interval.param2
  );

  mavlink::command_long_t request_once{{}};
  request_once.param1 = mavlink::attitude_MSG_ID;
  request_once.command = mavlink::MAV_CMD_REQUEST_MESSAGE;
  request_once.target_system = DRONE_SYSTEM_ID;
  request_once.target_component = DRONE_COMPONENT_ID;
  mavlink::command_long_serialize(request_once, payload);
  mavlink_frame_from_gcs(
    frame,
    2,
    mavlink::command_long_MSG_ID,
    mavlink::command_long_CRC_EXTRA,
    payload,
    mavlink::command_long_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  mavlink::command_long_parse(payload, request_once);

  mavlink::attitude_t attitude{{}};
  attitude.time_boot_ms = 12345;
  attitude.roll = 0.01f;
  attitude.pitch = -0.02f;
  attitude.yaw = 1.57f;
  mavlink::attitude_serialize(attitude, payload);
  mavlink_frame_from_drone(
    frame,
    3,
    mavlink::attitude_MSG_ID,
    mavlink::attitude_CRC_EXTRA,
    payload,
    mavlink::attitude_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

  mavlink::attitude_t parsed_attitude{{}};
  mavlink::attitude_parse(payload, parsed_attitude);
  std::printf(
    "  ATTITUDE roll=%f pitch=%f yaw=%f\n",
    parsed_attitude.roll,
    parsed_attitude.pitch,
    parsed_attitude.yaw
  );

  (void)dialect;
  return 0;
}}
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_var = dialect_var_name(dialect_stem);
    let dialect_type = dialect_type_name(dialect_stem);

    format!(
        r#"#include <cstdio>

#include "common.hpp"

struct simulated_param_t {{
  const char* id;
  float value;
  uint16_t index;
}};

/// Virtual parameter service for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
int main() {{
  mavlink::{dialect_type} dialect;
  mavlink::{dialect_var}_init(dialect);

  uint8_t payload[mavlink::param_value_ENCODED_LENGTH];
  mavlink::frame_t frame;

  mavlink::param_request_list_t list_request{{}};
  list_request.target_system = DRONE_SYSTEM_ID;
  list_request.target_component = DRONE_COMPONENT_ID;
  mavlink::param_request_list_serialize(list_request, payload);
  mavlink_frame_from_gcs(
    frame,
    1,
    mavlink::param_request_list_MSG_ID,
    mavlink::param_request_list_CRC_EXTRA,
    payload,
    mavlink::param_request_list_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  mavlink::param_request_list_parse(payload, list_request);

  const simulated_param_t simulated_params[] = {{
    {{ "SYSID_THISMAV", 1.0f, 0 }},
    {{ "SYSID_MYGCS", 255.0f, 1 }},
    {{ "COMPASS_ENABLE", 1.0f, 2 }},
  }};
  const size_t simulated_param_count =
    sizeof(simulated_params) / sizeof(simulated_params[0]);

  for (size_t i = 0; i < simulated_param_count; i++) {{
    mavlink::param_value_t value{{}};
    value.param_value = simulated_params[i].value;
    value.param_count = static_cast<uint16_t>(simulated_param_count);
    value.param_index = simulated_params[i].index;
    value.param_type = mavlink::MAV_PARAM_TYPE_REAL32;
    mavlink_param_id_from_string(value.param_id, simulated_params[i].id);

    mavlink::param_value_serialize(value, payload);
    mavlink_frame_from_drone(
      frame,
      static_cast<uint8_t>(simulated_params[i].index + 10),
      mavlink::param_value_MSG_ID,
      mavlink::param_value_CRC_EXTRA,
      payload,
      mavlink::param_value_ENCODED_LENGTH
    );
    mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

    mavlink::param_value_t parsed{{}};
    mavlink::param_value_parse(payload, parsed);
    char id_buf[17];
    mavlink_param_id_to_string(parsed.param_id, id_buf, sizeof(id_buf));
    std::printf(
      "  PARAM_VALUE [%zu/%zu] %s=%f\n",
      i + 1,
      simulated_param_count,
      id_buf,
      parsed.param_value
    );
  }}

  const char* param_name = "SYSID_THISMAV";
  mavlink::param_request_read_t read_request{{}};
  read_request.param_index = -1;
  read_request.target_system = DRONE_SYSTEM_ID;
  read_request.target_component = DRONE_COMPONENT_ID;
  mavlink_param_id_from_string(read_request.param_id, param_name);
  mavlink::param_request_read_serialize(read_request, payload);
  mavlink_frame_from_gcs(
    frame,
    50,
    mavlink::param_request_read_MSG_ID,
    mavlink::param_request_read_CRC_EXTRA,
    payload,
    mavlink::param_request_read_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

  mavlink::param_request_read_t parsed_read{{}};
  mavlink::param_request_read_parse(payload, parsed_read);
  char read_id_buf[17];
  mavlink_param_id_to_string(parsed_read.param_id, read_id_buf, sizeof(read_id_buf));
  std::printf("  PARAM_REQUEST_READ id=%s\n", read_id_buf);

  mavlink::param_value_t single_value{{}};
  single_value.param_value = 1.0f;
  single_value.param_count = static_cast<uint16_t>(simulated_param_count);
  single_value.param_index = 0;
  single_value.param_type = mavlink::MAV_PARAM_TYPE_REAL32;
  mavlink_param_id_from_string(single_value.param_id, param_name);
  mavlink::param_value_serialize(single_value, payload);
  mavlink_frame_from_drone(
    frame,
    51,
    mavlink::param_value_MSG_ID,
    mavlink::param_value_CRC_EXTRA,
    payload,
    mavlink::param_value_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);
  mavlink::param_value_parse(payload, single_value);

  (void)dialect;
  return 0;
}}
"#
    )
}
