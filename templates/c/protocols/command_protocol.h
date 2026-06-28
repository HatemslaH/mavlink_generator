#ifndef MAVLINK_PROTOCOLS_COMMAND_PROTOCOL_H
#define MAVLINK_PROTOCOLS_COMMAND_PROTOCOL_H

#include <stdint.h>

#include "../mavlink.h"
#include "mavlink_cancellation.h"
#include "mavlink_session.h"

typedef struct command_protocol command_protocol_t;
typedef struct command_server command_server_t;

typedef MAV_RESULT (*command_server_long_handler_fn)(const command_long_t *command, void *user_data);
typedef MAV_RESULT (*command_server_int_handler_fn)(const command_int_t *command, void *user_data);

command_protocol_t *command_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int default_timeout_ms
);

mavlink_wait_result_t command_protocol_send_long(
  command_protocol_t *protocol,
  const command_long_t *command,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_command_long(
  command_protocol_t *protocol,
  MAV_CMD command,
  float param1,
  float param2,
  float param3,
  float param4,
  float param5,
  float param6,
  float param7,
  uint8_t confirmation,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_request_message(
  command_protocol_t *protocol,
  uint32_t message_id,
  float param2,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_set_message_interval(
  command_protocol_t *protocol,
  uint32_t message_id,
  int32_t interval_us,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_stop_message_interval(
  command_protocol_t *protocol,
  uint32_t message_id,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_set_mission_current(
  command_protocol_t *protocol,
  uint16_t sequence,
  int reset_mission,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_arm(
  command_protocol_t *protocol,
  int force,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_disarm(
  command_protocol_t *protocol,
  int force,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_takeoff(
  command_protocol_t *protocol,
  double altitude,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_land(
  command_protocol_t *protocol,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_return_to_launch(
  command_protocol_t *protocol,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

mavlink_wait_result_t command_protocol_wait_for_ack(
  command_protocol_t *protocol,
  MAV_CMD command,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
);

void command_protocol_destroy(command_protocol_t *protocol);

command_server_t *command_server_create(
  mavlink_session_t *session,
  command_server_long_handler_fn on_long,
  command_server_int_handler_fn on_int,
  void *user_data
);

void command_server_close(command_server_t *server);
void command_server_destroy(command_server_t *server);

#endif
