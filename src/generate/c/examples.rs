use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct CExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/c/examples/README.md"),
    ),
    (
        "common.h",
        include_str!("../../../templates/c/examples/common.h"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

impl LanguageExampleGenerator for CExampleGenerator {
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
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.c")),
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
        r#"#include <stdio.h>

#include "common.h"

/// Example for the `{dialect_stem}` dialect: serialize a HEARTBEAT frame and
/// parse it back with [{dialect_type}].
int main(void) {{
  {dialect_type} dialect;
  {dialect_var}_init(&dialect);

  heartbeat_t heartbeat = {{
    .custom_mode = 0,
    .type = MAV_TYPE_QUADROTOR,
    .autopilot = MAV_AUTOPILOT_PX4,
    .base_mode = 0,
    .system_status = MAV_STATE_ACTIVE,
    .mavlink_version = dialect.base.version,
  }};

  uint8_t payload[heartbeat_ENCODED_LENGTH];
  heartbeat_serialize(&heartbeat, payload);

  mavlink_frame_t frame;
  mavlink_frame_from_gcs(
    &frame,
    0,
    heartbeat_MSG_ID,
    heartbeat_CRC_EXTRA,
    payload,
    heartbeat_ENCODED_LENGTH
  );

  uint8_t wire[MAVLINK_MAX_FRAME_SIZE];
  size_t wire_len = mavlink_frame_serialize_v2(&frame, wire, sizeof(wire));
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  printf("Serialized HEARTBEAT (%zu bytes)\n", wire_len);

  heartbeat_t parsed;
  dialect.base.parse(
    &dialect.base,
    heartbeat_MSG_ID,
    payload,
    heartbeat_ENCODED_LENGTH,
    &parsed
  );
  printf("Parsed HEARTBEAT type=%d status=%d\n", parsed.type, parsed.system_status);

  return 0;
}}
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_var = dialect_var_name(dialect_stem);
    let dialect_type = dialect_type_name(dialect_stem);

    format!(
        r#"#include <stdio.h>

#include "common.h"

/// Virtual mission upload for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
int main(void) {{
  {dialect_type} dialect;
  {dialect_var}_init(&dialect);

  const MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION;

  mission_item_t mission_items[2] = {{
    {{
      .param1 = 0,
      .param2 = 2,
      .param3 = 0,
      .param4 = 0,
      .x = 47.397742f,
      .y = 8.545594f,
      .z = 50,
      .seq = 0,
      .command = MAV_CMD_NAV_WAYPOINT,
      .target_system = DRONE_SYSTEM_ID,
      .target_component = DRONE_COMPONENT_ID,
      .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT,
      .current = 0,
      .autocontinue = 1,
      .mission_type = mission_type,
    }},
    {{
      .param1 = 0,
      .param2 = 2,
      .param3 = 0,
      .param4 = 0,
      .x = 47.398000f,
      .y = 8.546000f,
      .z = 50,
      .seq = 1,
      .command = MAV_CMD_NAV_WAYPOINT,
      .target_system = DRONE_SYSTEM_ID,
      .target_component = DRONE_COMPONENT_ID,
      .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT,
      .current = 0,
      .autocontinue = 1,
      .mission_type = mission_type,
    }},
  }};

  uint8_t payload[255];
  mavlink_frame_t frame;

  // 1. GCS announces mission size.
  mission_count_t count = {{
    .count = 2,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
    .mission_type = mission_type,
  }};
  mission_count_serialize(&count, payload);
  mavlink_frame_from_gcs(
    &frame,
    1,
    mission_count_MSG_ID,
    mission_count_CRC_EXTRA,
    payload,
    mission_count_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  mission_count_parse(payload, &count);

  // 2. Drone requests each mission item, GCS responds.
  for (uint16_t seq = 0; seq < 2; seq++) {{
    mission_request_t request = {{
      .seq = seq,
      .target_system = GCS_SYSTEM_ID,
      .target_component = GCS_COMPONENT_ID,
      .mission_type = mission_type,
    }};
    mission_request_serialize(&request, payload);
    mavlink_frame_from_drone(
      &frame,
      (uint8_t)(seq + 10),
      mission_request_MSG_ID,
      mission_request_CRC_EXTRA,
      payload,
      mission_request_ENCODED_LENGTH
    );
    mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);
    mission_request_parse(payload, &request);

    mission_item_t item = mission_items[seq];
    mission_item_serialize(&item, payload);
    mavlink_frame_from_gcs(
      &frame,
      (uint8_t)(seq + 20),
      mission_item_MSG_ID,
      mission_item_CRC_EXTRA,
      payload,
      mission_item_ENCODED_LENGTH
    );
    mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

    mission_item_t parsed_item;
    mission_item_parse(payload, &parsed_item);
    printf(
      "  uploaded seq=%u cmd=%u\n",
      parsed_item.seq,
      (unsigned)parsed_item.command
    );
  }}

  // 3. Drone accepts the mission.
  mission_ack_t ack = {{
    .target_system = GCS_SYSTEM_ID,
    .target_component = GCS_COMPONENT_ID,
    .type = MAV_MISSION_ACCEPTED,
    .mission_type = mission_type,
  }};
  mission_ack_serialize(&ack, payload);
  mavlink_frame_from_drone(
    &frame,
    99,
    mission_ack_MSG_ID,
    mission_ack_CRC_EXTRA,
    payload,
    mission_ack_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

  mission_ack_t parsed_ack;
  mission_ack_parse(payload, &parsed_ack);
  printf("Mission upload complete: %d\n", parsed_ack.type);

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
        r#"#include <stdio.h>

#include "common.h"

/// Virtual telemetry request for the `{dialect_stem}` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
int main(void) {{
  {dialect_type} dialect;
  {dialect_var}_init(&dialect);

  uint8_t payload[command_long_ENCODED_LENGTH];
  mavlink_frame_t frame;

  // Stream ATTITUDE (msg id 30) at 10 Hz (100_000 microseconds).
  command_long_t set_interval = {{
    .param1 = attitude_MSG_ID,
    .param2 = 100000,
    .param3 = 0,
    .param4 = 0,
    .param5 = 0,
    .param6 = 0,
    .param7 = 0,
    .command = MAV_CMD_SET_MESSAGE_INTERVAL,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
    .confirmation = 0,
  }};
  command_long_serialize(&set_interval, payload);
  mavlink_frame_from_gcs(
    &frame,
    1,
    command_long_MSG_ID,
    command_long_CRC_EXTRA,
    payload,
    command_long_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

  command_long_t parsed_interval;
  command_long_parse(payload, &parsed_interval);
  printf(
    "  SET_MESSAGE_INTERVAL msgId=%.0f interval_us=%.0f\n",
    parsed_interval.param1,
    parsed_interval.param2
  );

  // One-shot ATTITUDE sample via MAV_CMD_REQUEST_MESSAGE.
  command_long_t request_once = {{
    .param1 = attitude_MSG_ID,
    .param2 = 0,
    .param3 = 0,
    .param4 = 0,
    .param5 = 0,
    .param6 = 0,
    .param7 = 0,
    .command = MAV_CMD_REQUEST_MESSAGE,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
    .confirmation = 0,
  }};
  command_long_serialize(&request_once, payload);
  mavlink_frame_from_gcs(
    &frame,
    2,
    command_long_MSG_ID,
    command_long_CRC_EXTRA,
    payload,
    command_long_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  command_long_parse(payload, &request_once);

  // Simulated vehicle response: ATTITUDE telemetry frame.
  attitude_t attitude = {{
    .time_boot_ms = 12345,
    .roll = 0.01f,
    .pitch = -0.02f,
    .yaw = 1.57f,
    .rollspeed = 0,
    .pitchspeed = 0,
    .yawspeed = 0,
  }};
  attitude_serialize(&attitude, payload);
  mavlink_frame_from_drone(
    &frame,
    3,
    attitude_MSG_ID,
    attitude_CRC_EXTRA,
    payload,
    attitude_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

  attitude_t parsed_attitude;
  attitude_parse(payload, &parsed_attitude);
  printf(
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
        r#"#include <stdio.h>

#include "common.h"

typedef struct {{
  const char *id;
  float value;
  uint16_t index;
}} simulated_param_t;

/// Virtual parameter service for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
int main(void) {{
  {dialect_type} dialect;
  {dialect_var}_init(&dialect);

  uint8_t payload[param_value_ENCODED_LENGTH];
  mavlink_frame_t frame;

  // 1. GCS requests the full onboard parameter set.
  param_request_list_t list_request = {{
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
  }};
  param_request_list_serialize(&list_request, payload);
  mavlink_frame_from_gcs(
    &frame,
    1,
    param_request_list_MSG_ID,
    param_request_list_CRC_EXTRA,
    payload,
    param_request_list_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);
  param_request_list_parse(payload, &list_request);

  // 2. Drone responds with PARAM_VALUE messages (simulated subset).
  const simulated_param_t simulated_params[] = {{
    {{ "SYSID_THISMAV", 1.0f, 0 }},
    {{ "SYSID_MYGCS", 255.0f, 1 }},
    {{ "COMPASS_ENABLE", 1.0f, 2 }},
  }};
  const size_t simulated_param_count =
    sizeof(simulated_params) / sizeof(simulated_params[0]);

  for (size_t i = 0; i < simulated_param_count; i++) {{
    param_value_t value = {{
      .param_value = simulated_params[i].value,
      .param_count = (uint16_t)simulated_param_count,
      .param_index = simulated_params[i].index,
      .param_type = MAV_PARAM_TYPE_REAL32,
    }};
    mavlink_param_id_from_string(value.param_id, simulated_params[i].id);

    param_value_serialize(&value, payload);
    mavlink_frame_from_drone(
      &frame,
      (uint8_t)(simulated_params[i].index + 10),
      param_value_MSG_ID,
      param_value_CRC_EXTRA,
      payload,
      param_value_ENCODED_LENGTH
    );
    mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);

    param_value_t parsed;
    param_value_parse(payload, &parsed);
    char id_buf[17];
    mavlink_param_id_to_string(parsed.param_id, id_buf, sizeof(id_buf));
    printf(
      "  PARAM_VALUE [%zu/%zu] %s=%f\n",
      i + 1,
      simulated_param_count,
      id_buf,
      parsed.param_value
    );
  }}

  // 3. GCS requests one parameter by name (param_index = -1).
  const char *param_name = "SYSID_THISMAV";
  param_request_read_t read_request = {{
    .param_index = -1,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
  }};
  mavlink_param_id_from_string(read_request.param_id, param_name);
  param_request_read_serialize(&read_request, payload);
  mavlink_frame_from_gcs(
    &frame,
    50,
    param_request_read_MSG_ID,
    param_request_read_CRC_EXTRA,
    payload,
    param_request_read_ENCODED_LENGTH
  );
  mavlink_log_frame("GCS ->", frame.message_id, frame.system_id, frame.component_id);

  param_request_read_t parsed_read;
  param_request_read_parse(payload, &parsed_read);
  char read_id_buf[17];
  mavlink_param_id_to_string(parsed_read.param_id, read_id_buf, sizeof(read_id_buf));
  printf("  PARAM_REQUEST_READ id=%s\n", read_id_buf);

  // 4. Drone answers with the matching PARAM_VALUE.
  param_value_t single_value = {{
    .param_value = 1.0f,
    .param_count = (uint16_t)simulated_param_count,
    .param_index = 0,
    .param_type = MAV_PARAM_TYPE_REAL32,
  }};
  mavlink_param_id_from_string(single_value.param_id, param_name);
  param_value_serialize(&single_value, payload);
  mavlink_frame_from_drone(
    &frame,
    51,
    param_value_MSG_ID,
    param_value_CRC_EXTRA,
    payload,
    param_value_ENCODED_LENGTH
  );
  mavlink_log_frame("Drone ->", frame.message_id, frame.system_id, frame.component_id);
  param_value_parse(payload, &single_value);

  (void)dialect;
  return 0;
}}
"#
    )
}
