#include <stdio.h>

#include "common.h"

/// Example for the `rt_rc` dialect: serialize a HEARTBEAT frame and
/// parse it back with [mavlink_dialect_rt_rc_t].
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);

  heartbeat_t heartbeat = {
    .custom_mode = 0,
    .type = MAV_TYPE_QUADROTOR,
    .autopilot = MAV_AUTOPILOT_PX4,
    .base_mode = 0,
    .system_status = MAV_STATE_ACTIVE,
    .mavlink_version = dialect.base.version,
  };

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
}
