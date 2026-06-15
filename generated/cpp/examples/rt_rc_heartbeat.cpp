#include <cstdio>

#include "common.hpp"

/// Example for the `rt_rc` dialect: serialize a HEARTBEAT frame and
/// parse it back with [mavlink_dialect_rt_rc_t].
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  mavlink::heartbeat_t heartbeat{};
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

  mavlink::heartbeat_t parsed{};
  dialect.base.parse(
    &dialect.base,
    mavlink::heartbeat_MSG_ID,
    payload,
    mavlink::heartbeat_ENCODED_LENGTH,
    &parsed
  );
  std::printf("Parsed HEARTBEAT type=%d status=%d\n", static_cast<int>(parsed.type), static_cast<int>(parsed.system_status));

  return 0;
}
