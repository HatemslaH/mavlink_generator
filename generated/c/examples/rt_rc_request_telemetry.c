#include <stdio.h>

#include "common.h"

/// Virtual telemetry request for the `rt_rc` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);

  uint8_t payload[command_long_ENCODED_LENGTH];
  mavlink_frame_t frame;

  // Stream ATTITUDE (msg id 30) at 10 Hz (100_000 microseconds).
  command_long_t set_interval = {
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
  };
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
  command_long_t request_once = {
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
  };
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
  attitude_t attitude = {
    .time_boot_ms = 12345,
    .roll = 0.01f,
    .pitch = -0.02f,
    .yaw = 1.57f,
    .rollspeed = 0,
    .pitchspeed = 0,
    .yawspeed = 0,
  };
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
}
