#include <cstdio>

#include "common.hpp"

/// Virtual telemetry request for the `rt_rc` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  uint8_t payload[mavlink::command_long_ENCODED_LENGTH];
  mavlink::frame_t frame;

  mavlink::command_long_t set_interval{};
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

  mavlink::command_long_t parsed_interval{};
  mavlink::command_long_parse(payload, parsed_interval);
  std::printf(
    "  SET_MESSAGE_INTERVAL msgId=%.0f interval_us=%.0f\n",
    parsed_interval.param1,
    parsed_interval.param2
  );

  mavlink::command_long_t request_once{};
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

  mavlink::attitude_t attitude{};
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

  mavlink::attitude_t parsed_attitude{};
  mavlink::attitude_parse(payload, parsed_attitude);
  std::printf(
    "  ATTITUDE roll=%f pitch=%f yaw=%f\n",
    parsed_attitude.roll,
    parsed_attitude.pitch,
    parsed_attitude.yaw
  );

  (void)dialect;
  return 0;
}
