#include <cstdio>

#include "common.hpp"

/// Virtual mission upload for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  const auto mission_type = mavlink::MAV_MISSION_TYPE_MISSION;

  mavlink::mission_item_t mission_items[2] = {
    {
      0, 2, 0, 0,
      47.397742f, 8.545594f, 50,
      0,
      mavlink::MAV_CMD_NAV_WAYPOINT,
      DRONE_SYSTEM_ID,
      DRONE_COMPONENT_ID,
      mavlink::MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1,
      mission_type,
    },
    {
      0, 2, 0, 0,
      47.398000f, 8.546000f, 50,
      1,
      mavlink::MAV_CMD_NAV_WAYPOINT,
      DRONE_SYSTEM_ID,
      DRONE_COMPONENT_ID,
      mavlink::MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1,
      mission_type,
    },
  };

  uint8_t payload[255];
  mavlink::frame_t frame;

  mavlink::mission_count_t count{};
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

  for (uint16_t seq = 0; seq < 2; seq++) {
    mavlink::mission_request_t request{};
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

    mavlink::mission_item_t parsed_item{};
    mavlink::mission_item_parse(payload, parsed_item);
    std::printf(
      "  uploaded seq=%u cmd=%u\n",
      parsed_item.seq,
      static_cast<unsigned>(parsed_item.command)
    );
  }

  mavlink::mission_ack_t ack{};
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

  mavlink::mission_ack_t parsed_ack{};
  mavlink::mission_ack_parse(payload, parsed_ack);
  std::printf("Mission upload complete: %d\n", static_cast<int>(parsed_ack.type));

  (void)dialect;
  return 0;
}
