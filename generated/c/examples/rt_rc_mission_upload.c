#include <stdio.h>

#include "common.h"

/// Virtual mission upload for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);

  const MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION;

  mission_item_t mission_items[2] = {
    {
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
    },
    {
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
    },
  };

  uint8_t payload[255];
  mavlink_frame_t frame;

  // 1. GCS announces mission size.
  mission_count_t count = {
    .count = 2,
    .target_system = DRONE_SYSTEM_ID,
    .target_component = DRONE_COMPONENT_ID,
    .mission_type = mission_type,
  };
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
  for (uint16_t seq = 0; seq < 2; seq++) {
    mission_request_t request = {
      .seq = seq,
      .target_system = GCS_SYSTEM_ID,
      .target_component = GCS_COMPONENT_ID,
      .mission_type = mission_type,
    };
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
  }

  // 3. Drone accepts the mission.
  mission_ack_t ack = {
    .target_system = GCS_SYSTEM_ID,
    .target_component = GCS_COMPONENT_ID,
    .type = MAV_MISSION_ACCEPTED,
    .mission_type = mission_type,
  };
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
}
