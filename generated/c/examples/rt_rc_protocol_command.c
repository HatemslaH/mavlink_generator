#include <stdio.h>
#include "protocols_common.h"
static MAV_RESULT on_command_long(const command_long_t *command, void *user_data) {
  (void)user_data;
  printf("Vehicle received COMMAND_LONG: %u\n", (unsigned)command->command);
  return MAV_RESULT_ACCEPTED;
}
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_link_t link = virtual_mavlink_link_create(&dialect.base);
  command_server_t *command_server = command_server_create(link.drone, on_command_long, NULL, NULL);
  command_protocol_t *command_protocol = command_protocol_create(
    link.gcs, DRONE_SYSTEM_ID, DRONE_COMPONENT_ID, 5000);
  command_ack_t ack;
  command_protocol_set_message_interval(command_protocol, attitude_MSG_ID, 100000, 5000, NULL, &ack);
  printf("SET_MESSAGE_INTERVAL ack: %d\n", ack.result);
  command_protocol_request_message(command_protocol, attitude_MSG_ID, 0, 5000, NULL, &ack);
  printf("REQUEST_MESSAGE ack: %d\n", ack.result);
  command_protocol_arm(command_protocol, 0, 5000, NULL, &ack);
  printf("ARM ack: %d\n", ack.result);
  command_protocol_disarm(command_protocol, 0, 5000, NULL, &ack);
  printf("DISARM ack: %d\n", ack.result);
  command_server_destroy(command_server);
  command_protocol_destroy(command_protocol);
  virtual_mavlink_link_close(&link);
  return 0;
}
