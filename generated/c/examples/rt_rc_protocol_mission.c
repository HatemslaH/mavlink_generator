#include <stdio.h>
#include "protocols_common.h"
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_link_t link = virtual_mavlink_link_create(&dialect.base);
  mission_server_t *mission_server = mission_server_create(link.drone, MAV_MISSION_TYPE_MISSION);
  command_server_t *command_server = command_server_create(link.drone, NULL, NULL, NULL);
  mission_protocol_t *mission_protocol = mission_protocol_create(
    link.gcs, DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, 3000, 10000);
  mission_item_int_t plan[2] = {
    { .x = 473977420, .y = 85455940, .z = 50, .seq = 0, .command = MAV_CMD_NAV_WAYPOINT,
       .target_system = DRONE_SYSTEM_ID, .target_component = DRONE_COMPONENT_ID,
       .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT, .autocontinue = 1,
       .mission_type = MAV_MISSION_TYPE_MISSION },
    { .x = 473980000, .y = 85460000, .z = 50, .seq = 1, .command = MAV_CMD_NAV_WAYPOINT,
       .target_system = DRONE_SYSTEM_ID, .target_component = DRONE_COMPONENT_ID,
       .frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT, .autocontinue = 1,
       .mission_type = MAV_MISSION_TYPE_MISSION },
  };
  MAV_MISSION_RESULT upload_result;
  mission_protocol_upload(mission_protocol, plan, 2, MAV_MISSION_TYPE_MISSION,
    NULL, NULL, NULL, &upload_result);
  printf("Mission upload result: %d\n", upload_result);
  printf("Vehicle stored %zu items\n", mission_server_item_count(mission_server));
  mission_item_int_t downloaded[8];
  size_t downloaded_count = 0;
  mission_protocol_download(mission_protocol, downloaded, 8, &downloaded_count,
    MAV_MISSION_TYPE_MISSION, NULL, NULL, NULL);
  printf("Downloaded %zu mission items\n", downloaded_count);
  command_protocol_t *command_protocol = command_protocol_create(
    link.gcs, DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, 5000);
  mission_set_current_result_t set_current;
  mission_protocol_set_current_with_command(mission_protocol, 0, command_protocol, 1, 0, NULL, &set_current);
  printf("Set current seq=%u ack=%d\n", set_current.sequence, set_current.has_command_ack);
  MAV_MISSION_RESULT clear_result;
  mission_protocol_clear(mission_protocol, MAV_MISSION_TYPE_MISSION, NULL, &clear_result);
  printf("Mission clear result: %d\n", clear_result);
  command_server_destroy(command_server);
  mission_server_destroy(mission_server);
  mission_protocol_destroy(mission_protocol);
  command_protocol_destroy(command_protocol);
  virtual_mavlink_link_close(&link);
  return 0;
}
