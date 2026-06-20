#include <stdio.h>
#include "protocols_common.h"
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_link_t link = virtual_mavlink_link_create(&dialect.base);
  parameter_server_t *parameter_server = parameter_server_create(link.drone);
  parameter_server_set(parameter_server, "SYSID_THISMAV", 1, MAV_PARAM_TYPE_INT32);
  parameter_server_set(parameter_server, "SYSID_MYGCS", 255, MAV_PARAM_TYPE_INT32);
  parameter_server_set(parameter_server, "COMPASS_ENABLE", 1, MAV_PARAM_TYPE_INT32);
  parameter_protocol_t *parameter_protocol = parameter_protocol_create(
    link.gcs, DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, 500, 3000);
  param_entry_t entries[16];
  size_t count = 0;
  parameter_protocol_fetch_all(parameter_protocol, entries, 16, &count, NULL, NULL, NULL);
  printf("Fetched %zu parameters\n", count);
  param_entry_t single;
  parameter_protocol_read_by_name(parameter_protocol, "SYSID_THISMAV", &single, NULL);
  printf("Read SYSID_THISMAV=%f\n", single.value);
  param_entry_t updated;
  parameter_protocol_write_by_name(parameter_protocol, "COMPASS_ENABLE", 0, 0, &updated, NULL);
  printf("Wrote COMPASS_ENABLE=%f\n", updated.value);
  parameter_server_destroy(parameter_server);
  parameter_protocol_destroy(parameter_protocol);
  virtual_mavlink_link_close(&link);
  return 0;
}
