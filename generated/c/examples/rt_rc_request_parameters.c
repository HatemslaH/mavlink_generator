#include <stdio.h>

#include "common.h"

typedef struct {
  const char *id;
  float value;
  uint16_t index;
} simulated_param_t;

/// Virtual parameter service for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);

  uint8_t payload[param_value_ENCODED_LENGTH];
  mavlink_frame_t frame;

  // 1. GCS requests the full onboard parameter set.
  param_request_list_t list_request = {
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
  };
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
  const simulated_param_t simulated_params[] = {
    { "SYSID_THISMAV", 1.0f, 0 },
    { "SYSID_MYGCS", 255.0f, 1 },
    { "COMPASS_ENABLE", 1.0f, 2 },
  };
  const size_t simulated_param_count =
    sizeof(simulated_params) / sizeof(simulated_params[0]);

  for (size_t i = 0; i < simulated_param_count; i++) {
    param_value_t value = {
      .param_value = simulated_params[i].value,
      .param_count = (uint16_t)simulated_param_count,
      .param_index = simulated_params[i].index,
      .param_type = MAV_PARAM_TYPE_REAL32,
    };
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
  }

  // 3. GCS requests one parameter by name (param_index = -1).
  const char *param_name = "SYSID_THISMAV";
  param_request_read_t read_request = {
    .param_index = -1,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
  };
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
  param_value_t single_value = {
    .param_value = 1.0f,
    .param_count = (uint16_t)simulated_param_count,
    .param_index = 0,
    .param_type = MAV_PARAM_TYPE_REAL32,
  };
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
}
