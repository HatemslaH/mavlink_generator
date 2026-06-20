#include "command_protocol.h"

#include <stdlib.h>

struct command_protocol {
  mavlink_session_t *session;
  uint8_t target_system;
  uint8_t target_component;
  int default_timeout_ms;
};

struct command_server {
  mavlink_session_t *session;
  command_server_long_handler_fn on_long;
  command_server_int_handler_fn on_int;
  void *user_data;
  mavlink_message_subscription_t *subscription;
};

typedef struct {
  MAV_CMD command;
} command_ack_ctx_t;

static bool command_ack_predicate(const mavlink_frame_t *frame, void *user_data) {
  command_ack_ctx_t *ctx = (command_ack_ctx_t *)user_data;
  if (frame->message_id != command_ack_MSG_ID) {
    return false;
  }
  command_ack_t ack;
  command_ack_parse(frame->payload, &ack);
  return ack.command == ctx->command;
}

command_protocol_t *command_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int default_timeout_ms
) {
  command_protocol_t *protocol = (command_protocol_t *)calloc(1, sizeof(*protocol));
  if (protocol == NULL) {
    return NULL;
  }
  protocol->session = session;
  protocol->target_system = target_system;
  protocol->target_component = target_component;
  protocol->default_timeout_ms = default_timeout_ms > 0 ? default_timeout_ms : 5000;
  return protocol;
}

mavlink_wait_result_t command_protocol_send_long(
  command_protocol_t *protocol,
  const command_long_t *command,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  if (protocol == NULL || command == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  uint8_t payload[command_long_ENCODED_LENGTH];
  command_long_serialize(command, payload);
  if (mavlink_session_send(
        protocol->session,
        command_long_MSG_ID,
        command_long_CRC_EXTRA,
        payload,
        command_long_ENCODED_LENGTH) != 0) {
    return MAVLINK_WAIT_ERROR;
  }
  return command_protocol_wait_for_ack(protocol, command->command, timeout_ms, cancel, out_ack);
}

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
) {
  command_long_t cmd = {
    .param1 = param1,
    .param2 = param2,
    .param3 = param3,
    .param4 = param4,
    .param5 = param5,
    .param6 = param6,
    .param7 = param7,
    .command = command,
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .confirmation = confirmation,
  };
  return command_protocol_send_long(protocol, &cmd, timeout_ms, cancel, out_ack);
}

mavlink_wait_result_t command_protocol_request_message(
  command_protocol_t *protocol,
  uint32_t message_id,
  float param2,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_REQUEST_MESSAGE,
    (float)message_id,
    param2,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_set_message_interval(
  command_protocol_t *protocol,
  uint32_t message_id,
  int32_t interval_us,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_SET_MESSAGE_INTERVAL,
    (float)message_id,
    (float)interval_us,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_stop_message_interval(
  command_protocol_t *protocol,
  uint32_t message_id,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_set_message_interval(protocol, message_id, 0, timeout_ms, cancel, out_ack);
}

mavlink_wait_result_t command_protocol_set_mission_current(
  command_protocol_t *protocol,
  uint16_t sequence,
  int reset_mission,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_DO_SET_MISSION_CURRENT,
    (float)sequence,
    reset_mission ? 1.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_arm(
  command_protocol_t *protocol,
  int force,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_COMPONENT_ARM_DISARM,
    1.0f,
    force ? 21196.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_disarm(
  command_protocol_t *protocol,
  int force,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_COMPONENT_ARM_DISARM,
    0.0f,
    force ? 21196.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_takeoff(
  command_protocol_t *protocol,
  double altitude,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_NAV_TAKEOFF,
    0,
    0,
    0,
    0,
    0,
    0,
    (float)altitude,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_land(
  command_protocol_t *protocol,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_NAV_LAND,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_return_to_launch(
  command_protocol_t *protocol,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  return command_protocol_command_long(
    protocol,
    MAV_CMD_NAV_RETURN_TO_LAUNCH,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout_ms,
    cancel,
    out_ack
  );
}

mavlink_wait_result_t command_protocol_wait_for_ack(
  command_protocol_t *protocol,
  MAV_CMD command,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  command_ack_t *out_ack
) {
  if (protocol == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  if (timeout_ms <= 0) {
    timeout_ms = protocol->default_timeout_ms;
  }

  command_ack_ctx_t ctx = { command };
  mavlink_frame_t frame;
  return mavlink_session_wait_for_message(
    protocol->session,
    command_ack_predicate,
    &ctx,
    protocol->target_system,
    0,
    timeout_ms,
    cancel,
    &frame,
    out_ack,
    out_ack != NULL ? sizeof(*out_ack) : 0
  );
}

void command_protocol_destroy(command_protocol_t *protocol) {
  free(protocol);
}

static void command_server_on_frame(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  command_server_t *server = (command_server_t *)user_data;
  if (server == NULL || parsed_message == NULL) {
    return;
  }

  if (frame->message_id == command_long_MSG_ID) {
    const command_long_t *command = (const command_long_t *)parsed_message;
    if (command->target_system != mavlink_session_system_id(server->session)) {
      return;
    }
    MAV_RESULT result = MAV_RESULT_ACCEPTED;
    if (server->on_long != NULL) {
      result = server->on_long(command, server->user_data);
    }
    command_ack_t ack = {
      .command = command->command,
      .result = result,
      .progress = 0,
      .result_param2 = 0,
      .target_system = frame->system_id,
      .target_component = frame->component_id,
    };
    uint8_t payload[command_ack_ENCODED_LENGTH];
    command_ack_serialize(&ack, payload);
    mavlink_session_send(
      server->session,
      command_ack_MSG_ID,
      command_ack_CRC_EXTRA,
      payload,
      command_ack_ENCODED_LENGTH
    );
    return;
  }

  if (frame->message_id == command_int_MSG_ID) {
    const command_int_t *command = (const command_int_t *)parsed_message;
    if (command->target_system != mavlink_session_system_id(server->session)) {
      return;
    }
    MAV_RESULT result = MAV_RESULT_ACCEPTED;
    if (server->on_int != NULL) {
      result = server->on_int(command, server->user_data);
    }
    command_ack_t ack = {
      .command = command->command,
      .result = result,
      .progress = 0,
      .result_param2 = 0,
      .target_system = frame->system_id,
      .target_component = frame->component_id,
    };
    uint8_t payload[command_ack_ENCODED_LENGTH];
    command_ack_serialize(&ack, payload);
    mavlink_session_send(
      server->session,
      command_ack_MSG_ID,
      command_ack_CRC_EXTRA,
      payload,
      command_ack_ENCODED_LENGTH
    );
  }
}

command_server_t *command_server_create(
  mavlink_session_t *session,
  command_server_long_handler_fn on_long,
  command_server_int_handler_fn on_int,
  void *user_data
) {
  command_server_t *server = (command_server_t *)calloc(1, sizeof(*server));
  if (server == NULL) {
    return NULL;
  }
  server->session = session;
  server->on_long = on_long;
  server->on_int = on_int;
  server->user_data = user_data;
  server->subscription = mavlink_session_listen_message(session, 0, 0, 0, command_server_on_frame, server);
  return server;
}

void command_server_close(command_server_t *server) {
  if (server != NULL && server->subscription != NULL) {
    mavlink_message_subscription_cancel(server->subscription);
    server->subscription = NULL;
  }
}

void command_server_destroy(command_server_t *server) {
  if (server == NULL) {
    return;
  }
  command_server_close(server);
  free(server);
}
