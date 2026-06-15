#include <cstdio>

#include "common.hpp"

struct simulated_param_t {
  const char* id;
  float value;
  uint16_t index;
};

/// Virtual parameter service for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  uint8_t payload[mavlink::param_value_ENCODED_LENGTH];
  mavlink::frame_t frame;

  mavlink::param_request_list_t list_request{};
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

  const simulated_param_t simulated_params[] = {
    { "SYSID_THISMAV", 1.0f, 0 },
    { "SYSID_MYGCS", 255.0f, 1 },
    { "COMPASS_ENABLE", 1.0f, 2 },
  };
  const size_t simulated_param_count =
    sizeof(simulated_params) / sizeof(simulated_params[0]);

  for (size_t i = 0; i < simulated_param_count; i++) {
    mavlink::param_value_t value{};
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

    mavlink::param_value_t parsed{};
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
  }

  const char* param_name = "SYSID_THISMAV";
  mavlink::param_request_read_t read_request{};
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

  mavlink::param_request_read_t parsed_read{};
  mavlink::param_request_read_parse(payload, parsed_read);
  char read_id_buf[17];
  mavlink_param_id_to_string(parsed_read.param_id, read_id_buf, sizeof(read_id_buf));
  std::printf("  PARAM_REQUEST_READ id=%s\n", read_id_buf);

  mavlink::param_value_t single_value{};
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
}
