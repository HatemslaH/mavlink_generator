#include "sample_mission.h"

#include <stdio.h>

size_t build_sample_mission(
  mission_item_int_t *out_items,
  size_t max_items,
  uint8_t target_system,
  uint8_t target_component
) {
  if (out_items == NULL || max_items < 3) {
    return 0;
  }

  out_items[0] = (mission_item_int_t){
    .x = 473977420,
    .y = 85455940,
    .z = 50.0f,
    .seq = 0,
    .command = MAV_CMD_NAV_WAYPOINT,
    .target_system = target_system,
    .target_component = target_component,
    .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
    .autocontinue = 1,
    .mission_type = MAV_MISSION_TYPE_MISSION,
  };
  out_items[1] = (mission_item_int_t){
    .x = 473980000,
    .y = 85460000,
    .z = 50.0f,
    .seq = 1,
    .command = MAV_CMD_NAV_WAYPOINT,
    .target_system = target_system,
    .target_component = target_component,
    .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
    .autocontinue = 1,
    .mission_type = MAV_MISSION_TYPE_MISSION,
  };
  out_items[2] = (mission_item_int_t){
    .x = 473982580,
    .y = 85464060,
    .z = 50.0f,
    .seq = 2,
    .command = MAV_CMD_NAV_RETURN_TO_LAUNCH,
    .target_system = target_system,
    .target_component = target_component,
    .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
    .autocontinue = 1,
    .mission_type = MAV_MISSION_TYPE_MISSION,
  };
  return 3;
}

void describe_mission_item(const mission_item_int_t *item, char *buf, size_t buf_len) {
  if (item == NULL || buf == NULL || buf_len == 0) {
    return;
  }
  double lat = item->x / 1e7;
  double lon = item->y / 1e7;
  snprintf(
    buf,
    buf_len,
    "seq=%u cmd=%d lat=%.6f lon=%.6f alt=%.0fm",
    item->seq,
    (int)item->command,
    lat,
    lon,
    (double)item->z
  );
}
