#include <stdio.h>
#include "protocols_common.h"
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_bus_t *bus = virtual_mavlink_bus_create();
  mavlink_gcs_t *gcs = mavlink_gcs_connect(
    &dialect.base, virtual_mavlink_bus_create_endpoint(bus),
    GCS_SYSTEM_ID, GCS_COMPONENT_ID, 500, 3000, MAVLINK_VERSION_V2);
  mavlink_session_t *drone_session = mavlink_session_create(
    &dialect.base, virtual_mavlink_bus_create_endpoint(bus),
    DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, MAVLINK_VERSION_V2);
  heartbeat_t drone_hb = {
    .type = MAV_TYPE_QUADROTOR, .autopilot = MAV_AUTOPILOT_PX4,
    .system_status = MAV_STATE_ACTIVE, .mavlink_version = dialect.base.version,
  };
  uint8_t drone_payload[heartbeat_ENCODED_LENGTH];
  heartbeat_serialize(&drone_hb, drone_payload);
  heartbeat_publisher_t *drone_publisher = heartbeat_publisher_create(
    drone_session, drone_payload, heartbeat_MSG_ID, heartbeat_CRC_EXTRA,
    heartbeat_ENCODED_LENGTH, 500);
  parameter_server_t *parameter_server = parameter_server_create(drone_session);
  parameter_server_set(parameter_server, "SYSID_THISMAV", 1, MAV_PARAM_TYPE_INT32);
  command_server_t *command_server = command_server_create(drone_session, NULL, NULL, NULL);
  mavlink_gcs_start(gcs);
  heartbeat_publisher_start(drone_publisher);
  uint8_t exclude[] = { GCS_SYSTEM_ID };
  mavlink_vehicle_client_t *client = NULL;
  mavlink_gcs_wait_for_vehicle(gcs, exclude, 1, 60000, &client);
  printf("Connected to vehicle %u:%u\n", client->vehicle.system_id, client->vehicle.component_id);
  param_entry_t params[8];
  size_t param_count = 0;
  parameter_protocol_fetch_all(client->parameters, params, 8, &param_count, NULL, NULL, NULL);
  printf("Vehicle has %zu parameters\n", param_count);
  command_ack_t ack;
  command_protocol_request_message(client->command, heartbeat_MSG_ID, 0, 10000, NULL, &ack);
  printf("REQUEST_MESSAGE ack: %d\n", ack.result);
  mavlink_vehicle_client_destroy(client);
  command_server_destroy(command_server);
  parameter_server_destroy(parameter_server);
  heartbeat_publisher_destroy(drone_publisher);
  mavlink_session_close(drone_session);
  mavlink_gcs_destroy(gcs);
  virtual_mavlink_bus_close_all(bus);
  return 0;
}
