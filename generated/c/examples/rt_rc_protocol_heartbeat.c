#include <stdio.h>
#include "protocols_common.h"
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_link_t link = virtual_mavlink_link_create(&dialect.base);
  heartbeat_t drone_hb = {
    .type = MAV_TYPE_QUADROTOR, .autopilot = MAV_AUTOPILOT_PX4,
    .system_status = MAV_STATE_ACTIVE, .mavlink_version = dialect.base.version,
  };
  uint8_t drone_payload[heartbeat_ENCODED_LENGTH];
  heartbeat_serialize(&drone_hb, drone_payload);
  heartbeat_publisher_t *drone_publisher = heartbeat_publisher_create(
    link.drone, drone_payload, heartbeat_MSG_ID, heartbeat_CRC_EXTRA,
    heartbeat_ENCODED_LENGTH, 500);
  heartbeat_monitor_t *gcs_monitor = heartbeat_monitor_create(link.gcs, 2000);
  heartbeat_monitor_start(gcs_monitor);
  heartbeat_publisher_start(drone_publisher);
  uint8_t exclude[] = { GCS_SYSTEM_ID };
  mavlink_node_t vehicle;
  heartbeat_monitor_wait_for_vehicle(gcs_monitor, exclude, 1, 5000, NULL, &vehicle);
  printf("Vehicle discovered: sys=%u comp=%u\n", vehicle.system_id, vehicle.component_id);
  printf("Drone online: %d\n", heartbeat_monitor_is_online(gcs_monitor, vehicle));
  heartbeat_publisher_stop(drone_publisher);
  heartbeat_monitor_stop(gcs_monitor);
  heartbeat_publisher_destroy(drone_publisher);
  heartbeat_monitor_destroy(gcs_monitor);
  virtual_mavlink_link_close(&link);
  return 0;
}
