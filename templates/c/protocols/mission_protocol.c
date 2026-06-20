#include "mission_protocol.h"

#include <stdlib.h>
#include <string.h>

static void mission_items_resequence(mission_item_int_t *items, size_t count) {
  for (size_t i = 0; i < count; i++) {
    items[i].seq = (uint16_t)i;
  }
}

struct mission_protocol {
  mavlink_session_t *session;
  uint8_t target_system;
  uint8_t target_component;
  int item_timeout_ms;
  int operation_timeout_ms;
};

struct mission_server {
  mavlink_session_t *session;
  MAV_MISSION_TYPE mission_type;
  mission_item_int_t items[MAVLINK_MISSION_MAX_ITEMS];
  size_t item_count;
  mission_item_int_t incoming[MAVLINK_MISSION_MAX_ITEMS];
  int incoming_count;
  int incoming_expected;
  mavlink_message_subscription_t *subscription;
};

static bool mission_protocol_is_item_request(const mavlink_frame_t *frame, uint16_t seq, MAV_MISSION_TYPE mission_type) {
  if (frame->message_id == mission_request_int_MSG_ID) {
    mission_request_int_t request;
    mission_request_int_parse(frame->payload, &request);
    return request.seq == seq && request.mission_type == mission_type;
  }
  if (frame->message_id == mission_request_MSG_ID) {
    mission_request_t request;
    mission_request_parse(frame->payload, &request);
    return request.seq == seq && request.mission_type == mission_type;
  }
  return false;
}

typedef struct {
  uint16_t seq;
  MAV_MISSION_TYPE mission_type;
} mission_request_ctx_t;

static bool mission_protocol_request_predicate(const mavlink_frame_t *frame, void *user_data) {
  mission_request_ctx_t *ctx = (mission_request_ctx_t *)user_data;
  return mission_protocol_is_item_request(frame, ctx->seq, ctx->mission_type);
}

mission_protocol_t *mission_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int item_timeout_ms,
  int operation_timeout_ms
) {
  mission_protocol_t *protocol = (mission_protocol_t *)calloc(1, sizeof(*protocol));
  if (protocol == NULL) {
    return NULL;
  }
  protocol->session = session;
  protocol->target_system = target_system;
  protocol->target_component = target_component;
  protocol->item_timeout_ms = item_timeout_ms > 0 ? item_timeout_ms : 3000;
  protocol->operation_timeout_ms = operation_timeout_ms > 0 ? operation_timeout_ms : 10000;
  return protocol;
}

mavlink_wait_result_t mission_protocol_upload(
  mission_protocol_t *protocol,
  const mission_item_int_t *items,
  size_t item_count,
  MAV_MISSION_TYPE mission_type,
  mission_upload_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel,
  MAV_MISSION_RESULT *out_result
) {
  if (protocol == NULL || items == NULL || item_count == 0) {
    return MAVLINK_WAIT_ERROR;
  }
  if (mavlink_cancellation_token_throw_if_cancelled(cancel) != 0) {
    return MAVLINK_WAIT_CANCELLED;
  }

  mission_item_int_t plan[MAVLINK_MISSION_MAX_ITEMS];
  if (item_count > MAVLINK_MISSION_MAX_ITEMS) {
    return MAVLINK_WAIT_ERROR;
  }
  memcpy(plan, items, item_count * sizeof(mission_item_int_t));
  mission_items_resequence(plan, item_count);

  mission_count_t count = {
    .count = (uint16_t)item_count,
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .mission_type = mission_type,
  };
  uint8_t payload[mission_count_ENCODED_LENGTH];
  mission_count_serialize(&count, payload);
  mavlink_session_send(
    protocol->session,
    mission_count_MSG_ID,
    mission_count_CRC_EXTRA,
    payload,
    mission_count_ENCODED_LENGTH
  );

  for (size_t i = 0; i < item_count; i++) {
    if (mavlink_cancellation_token_throw_if_cancelled(cancel) != 0) {
      return MAVLINK_WAIT_CANCELLED;
    }

    mission_request_ctx_t ctx = { plan[i].seq, mission_type };
    mavlink_frame_t frame;
    mavlink_wait_result_t wait = mavlink_session_wait_for_message(
      protocol->session,
      mission_protocol_request_predicate,
      &ctx,
      protocol->target_system,
      0,
      protocol->item_timeout_ms,
      cancel,
      &frame,
      NULL,
      0
    );
    if (wait != MAVLINK_WAIT_OK) {
      return wait;
    }

    if (frame.message_id == mission_request_int_MSG_ID) {
      uint8_t item_payload[mission_item_int_ENCODED_LENGTH];
      mission_item_int_serialize(&plan[i], item_payload);
      mavlink_session_send(
        protocol->session,
        mission_item_int_MSG_ID,
        mission_item_int_CRC_EXTRA,
        item_payload,
        mission_item_int_ENCODED_LENGTH
      );
    } else {
      mission_item_t legacy;
      legacy.param1 = plan[i].param1;
      legacy.param2 = plan[i].param2;
      legacy.param3 = plan[i].param3;
      legacy.param4 = plan[i].param4;
      legacy.x = (float)plan[i].x / 1e7f;
      legacy.y = (float)plan[i].y / 1e7f;
      legacy.z = plan[i].z;
      legacy.seq = plan[i].seq;
      legacy.command = plan[i].command;
      legacy.target_system = plan[i].target_system;
      legacy.target_component = plan[i].target_component;
      legacy.frame = plan[i].frame;
      legacy.current = plan[i].current;
      legacy.autocontinue = plan[i].autocontinue;
      legacy.mission_type = plan[i].mission_type;
      uint8_t item_payload[mission_item_ENCODED_LENGTH];
      mission_item_serialize(&legacy, item_payload);
      mavlink_session_send(
        protocol->session,
        mission_item_MSG_ID,
        mission_item_CRC_EXTRA,
        item_payload,
        mission_item_ENCODED_LENGTH
      );
    }

    if (on_progress != NULL) {
      on_progress((int)i + 1, (int)item_count, &plan[i], progress_ctx);
    }
  }

  mavlink_frame_t ack_frame;
  mission_ack_t ack;
  mavlink_wait_result_t wait = mavlink_session_wait_for_message_id(
    protocol->session,
    mission_ack_MSG_ID,
    protocol->target_system,
    0,
    protocol->operation_timeout_ms,
    cancel,
    &ack_frame,
    &ack,
    sizeof(ack)
  );
  if (wait != MAVLINK_WAIT_OK) {
    return wait;
  }
  if (out_result != NULL) {
    *out_result = ack.type;
  }
  return MAVLINK_WAIT_OK;
}

mavlink_wait_result_t mission_protocol_download(
  mission_protocol_t *protocol,
  mission_item_int_t *out_items,
  size_t max_items,
  size_t *out_count,
  MAV_MISSION_TYPE mission_type,
  mission_download_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel
) {
  if (protocol == NULL || out_items == NULL) {
    return MAVLINK_WAIT_ERROR;
  }

  mission_request_list_t list = {
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .mission_type = mission_type,
  };
  uint8_t payload[mission_request_list_ENCODED_LENGTH];
  mission_request_list_serialize(&list, payload);
  mavlink_session_send(
    protocol->session,
    mission_request_list_MSG_ID,
    mission_request_list_CRC_EXTRA,
    payload,
    mission_request_list_ENCODED_LENGTH
  );

  mavlink_frame_t count_frame;
  mission_count_t count_msg;
  mavlink_wait_result_t wait = mavlink_session_wait_for_message_id(
    protocol->session,
    mission_count_MSG_ID,
    protocol->target_system,
    0,
    protocol->operation_timeout_ms,
    cancel,
    &count_frame,
    &count_msg,
    sizeof(count_msg)
  );
  if (wait != MAVLINK_WAIT_OK) {
    return wait;
  }

  size_t received = 0;
  for (uint16_t seq = 0; seq < count_msg.count; seq++) {
    if (mavlink_cancellation_token_throw_if_cancelled(cancel) != 0) {
      return MAVLINK_WAIT_CANCELLED;
    }
    if (received >= max_items) {
      return MAVLINK_WAIT_ERROR;
    }

    mission_request_int_t request = {
      .seq = seq,
      .target_system = protocol->target_system,
      .target_component = protocol->target_component,
      .mission_type = mission_type,
    };
    uint8_t req_payload[mission_request_int_ENCODED_LENGTH];
    mission_request_int_serialize(&request, req_payload);
    mavlink_session_send(
      protocol->session,
      mission_request_int_MSG_ID,
      mission_request_int_CRC_EXTRA,
      req_payload,
      mission_request_int_ENCODED_LENGTH
    );

    mavlink_frame_t item_frame;
    mission_item_int_t item;
    wait = mavlink_session_wait_for_message_id(
      protocol->session,
      mission_item_int_MSG_ID,
      protocol->target_system,
      0,
      protocol->item_timeout_ms,
      cancel,
      &item_frame,
      &item,
      sizeof(item)
    );
    if (wait != MAVLINK_WAIT_OK) {
      if (wait == MAVLINK_WAIT_TIMEOUT) {
        mission_item_t legacy;
        wait = mavlink_session_wait_for_message_id(
          protocol->session,
          mission_item_MSG_ID,
          protocol->target_system,
          0,
          protocol->item_timeout_ms,
          cancel,
          &item_frame,
          &legacy,
          sizeof(legacy)
        );
        if (wait != MAVLINK_WAIT_OK) {
          return wait;
        }
        item.param1 = legacy.param1;
        item.param2 = legacy.param2;
        item.param3 = legacy.param3;
        item.param4 = legacy.param4;
        item.x = (int32_t)(legacy.x * 1e7f);
        item.y = (int32_t)(legacy.y * 1e7f);
        item.z = legacy.z;
        item.seq = legacy.seq;
        item.command = legacy.command;
        item.target_system = legacy.target_system;
        item.target_component = legacy.target_component;
        item.frame = legacy.frame;
        item.current = legacy.current;
        item.autocontinue = legacy.autocontinue;
        item.mission_type = legacy.mission_type;
      } else {
        return wait;
      }
    }

    out_items[received++] = item;
    if (on_progress != NULL) {
      on_progress((int)received, count_msg.count, &item, progress_ctx);
    }
  }

  mission_ack_t ack = {
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .type = MAV_MISSION_ACCEPTED,
    .mission_type = mission_type,
  };
  uint8_t ack_payload[mission_ack_ENCODED_LENGTH];
  mission_ack_serialize(&ack, ack_payload);
  mavlink_session_send(
    protocol->session,
    mission_ack_MSG_ID,
    mission_ack_CRC_EXTRA,
    ack_payload,
    mission_ack_ENCODED_LENGTH
  );

  if (out_count != NULL) {
    *out_count = received;
  }
  return MAVLINK_WAIT_OK;
}

mavlink_wait_result_t mission_protocol_clear(
  mission_protocol_t *protocol,
  MAV_MISSION_TYPE mission_type,
  mavlink_cancellation_token_t *cancel,
  MAV_MISSION_RESULT *out_result
) {
  mission_clear_all_t clear = {
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .mission_type = mission_type,
  };
  uint8_t payload[mission_clear_all_ENCODED_LENGTH];
  mission_clear_all_serialize(&clear, payload);
  mavlink_session_send(
    protocol->session,
    mission_clear_all_MSG_ID,
    mission_clear_all_CRC_EXTRA,
    payload,
    mission_clear_all_ENCODED_LENGTH
  );

  mavlink_frame_t frame;
  mission_ack_t ack;
  mavlink_wait_result_t wait = mavlink_session_wait_for_message_id(
    protocol->session,
    mission_ack_MSG_ID,
    protocol->target_system,
    0,
    protocol->operation_timeout_ms,
    cancel,
    &frame,
    &ack,
    sizeof(ack)
  );
  if (wait == MAVLINK_WAIT_OK && out_result != NULL) {
    *out_result = ack.type;
  }
  return wait;
}

mavlink_wait_result_t mission_protocol_set_current(
  mission_protocol_t *protocol,
  uint16_t seq,
  mavlink_cancellation_token_t *cancel
) {
  (void)cancel;
  mission_set_current_t current = {
    .seq = seq,
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
  };
  uint8_t payload[mission_set_current_ENCODED_LENGTH];
  mission_set_current_serialize(&current, payload);
  return mavlink_session_send(
           protocol->session,
           mission_set_current_MSG_ID,
           mission_set_current_CRC_EXTRA,
           payload,
           mission_set_current_ENCODED_LENGTH
         ) == 0
    ? MAVLINK_WAIT_OK
    : MAVLINK_WAIT_ERROR;
}

mavlink_wait_result_t mission_protocol_set_current_with_command(
  mission_protocol_t *protocol,
  uint16_t seq,
  command_protocol_t *command,
  int also_send_command,
  int reset_mission,
  mavlink_cancellation_token_t *cancel,
  mission_set_current_result_t *out_result
) {
  mavlink_wait_result_t result = mission_protocol_set_current(protocol, seq, cancel);
  if (result != MAVLINK_WAIT_OK) {
    return result;
  }

  if (out_result != NULL) {
    out_result->sequence = seq;
    out_result->has_command_ack = 0;
  }

  if (also_send_command && command != NULL) {
    command_ack_t ack;
    result = command_protocol_set_mission_current(command, seq, reset_mission, cancel, &ack);
    if (result == MAVLINK_WAIT_OK && out_result != NULL) {
      out_result->has_command_ack = 1;
      out_result->command_ack = ack;
    }
    return result;
  }
  return MAVLINK_WAIT_OK;
}

void mission_protocol_destroy(mission_protocol_t *protocol) {
  free(protocol);
}

static bool mission_server_targets_us(mission_server_t *server, uint8_t target_system, uint8_t target_component) {
  if (target_system != mavlink_session_system_id(server->session) && target_system != 0) {
    return false;
  }
  if (target_component != mavlink_session_component_id(server->session) && target_component != 0) {
    return false;
  }
  return true;
}

static void mission_server_send_request(
  mission_server_t *server,
  uint8_t target_system,
  uint8_t target_component,
  uint16_t seq
) {
  mission_request_int_t request = {
    .seq = seq,
    .target_system = target_system,
    .target_component = target_component,
    .mission_type = server->mission_type,
  };
  uint8_t payload[mission_request_int_ENCODED_LENGTH];
  mission_request_int_serialize(&request, payload);
  mavlink_session_send(
    server->session,
    mission_request_int_MSG_ID,
    mission_request_int_CRC_EXTRA,
    payload,
    mission_request_int_ENCODED_LENGTH
  );
}

static void mission_server_send_ack(
  mission_server_t *server,
  uint8_t target_system,
  uint8_t target_component,
  MAV_MISSION_RESULT result
) {
  mission_ack_t ack = {
    .target_system = target_system,
    .target_component = target_component,
    .type = result,
    .mission_type = server->mission_type,
  };
  uint8_t payload[mission_ack_ENCODED_LENGTH];
  mission_ack_serialize(&ack, payload);
  mavlink_session_send(
    server->session,
    mission_ack_MSG_ID,
    mission_ack_CRC_EXTRA,
    payload,
    mission_ack_ENCODED_LENGTH
  );
}

static void mission_server_on_frame(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  mission_server_t *server = (mission_server_t *)user_data;
  if (server == NULL || parsed_message == NULL) {
    return;
  }

  if (frame->message_id == mission_count_MSG_ID) {
    const mission_count_t *count = (const mission_count_t *)parsed_message;
    if (!mission_server_targets_us(server, count->target_system, count->target_component) ||
        count->mission_type != server->mission_type) {
      return;
    }
    server->incoming_expected = count->count;
    server->incoming_count = 0;
    if (count->count > 0) {
      mission_server_send_request(server, frame->system_id, frame->component_id, 0);
    } else {
      mission_server_send_ack(server, frame->system_id, frame->component_id, MAV_MISSION_ACCEPTED);
    }
    return;
  }

  if (frame->message_id == mission_item_int_MSG_ID) {
    const mission_item_int_t *item = (const mission_item_int_t *)parsed_message;
    if (!mission_server_targets_us(server, item->target_system, item->target_component) ||
        item->mission_type != server->mission_type) {
      return;
    }
    if (item->seq < MAVLINK_MISSION_MAX_ITEMS) {
      server->incoming[item->seq] = *item;
      server->incoming_count++;
    }
    if (server->incoming_expected > 0 && server->incoming_count < server->incoming_expected) {
      mission_server_send_request(server, frame->system_id, frame->component_id, (uint16_t)(item->seq + 1));
      return;
    }
    server->item_count = (size_t)server->incoming_expected;
    memcpy(server->items, server->incoming, server->item_count * sizeof(mission_item_int_t));
    server->incoming_count = 0;
    server->incoming_expected = 0;
    mission_server_send_ack(server, frame->system_id, frame->component_id, MAV_MISSION_ACCEPTED);
    return;
  }

  if (frame->message_id == mission_request_int_MSG_ID || frame->message_id == mission_request_MSG_ID) {
    uint16_t seq = 0;
    uint8_t target_system = 0;
    uint8_t target_component = 0;
    if (frame->message_id == mission_request_int_MSG_ID) {
      const mission_request_int_t *request = (const mission_request_int_t *)parsed_message;
      seq = request->seq;
      target_system = request->target_system;
      target_component = request->target_component;
    } else {
      const mission_request_t *request = (const mission_request_t *)parsed_message;
      seq = request->seq;
      target_system = request->target_system;
      target_component = request->target_component;
    }
    if (!mission_server_targets_us(server, target_system, target_component)) {
      return;
    }
    if (seq >= server->item_count) {
      mission_server_send_ack(server, frame->system_id, frame->component_id, MAV_MISSION_INVALID_SEQUENCE);
      return;
    }
    uint8_t payload[mission_item_int_ENCODED_LENGTH];
    mission_item_int_serialize(&server->items[seq], payload);
    mavlink_session_send(
      server->session,
      mission_item_int_MSG_ID,
      mission_item_int_CRC_EXTRA,
      payload,
      mission_item_int_ENCODED_LENGTH
    );
    return;
  }

  if (frame->message_id == mission_request_list_MSG_ID) {
    const mission_request_list_t *list = (const mission_request_list_t *)parsed_message;
    if (!mission_server_targets_us(server, list->target_system, list->target_component) ||
        list->mission_type != server->mission_type) {
      return;
    }
    mission_count_t count = {
      .count = (uint16_t)server->item_count,
      .target_system = frame->system_id,
      .target_component = frame->component_id,
      .mission_type = server->mission_type,
    };
    uint8_t payload[mission_count_ENCODED_LENGTH];
    mission_count_serialize(&count, payload);
    mavlink_session_send(
      server->session,
      mission_count_MSG_ID,
      mission_count_CRC_EXTRA,
      payload,
      mission_count_ENCODED_LENGTH
    );
    return;
  }

  if (frame->message_id == mission_clear_all_MSG_ID) {
    const mission_clear_all_t *clear = (const mission_clear_all_t *)parsed_message;
    if (!mission_server_targets_us(server, clear->target_system, clear->target_component) ||
        clear->mission_type != server->mission_type) {
      return;
    }
    server->item_count = 0;
    server->incoming_count = 0;
    server->incoming_expected = 0;
    mission_server_send_ack(server, frame->system_id, frame->component_id, MAV_MISSION_ACCEPTED);
  }
}

mission_server_t *mission_server_create(mavlink_session_t *session, MAV_MISSION_TYPE mission_type) {
  mission_server_t *server = (mission_server_t *)calloc(1, sizeof(*server));
  if (server == NULL) {
    return NULL;
  }
  server->session = session;
  server->mission_type = mission_type;
  server->subscription = mavlink_session_listen_message(session, 0, 0, 0, mission_server_on_frame, server);
  return server;
}

void mission_server_replace_mission(mission_server_t *server, const mission_item_int_t *items, size_t item_count) {
  if (server == NULL || items == NULL) {
    return;
  }
  if (item_count > MAVLINK_MISSION_MAX_ITEMS) {
    item_count = MAVLINK_MISSION_MAX_ITEMS;
  }
  memcpy(server->items, items, item_count * sizeof(mission_item_int_t));
  mission_items_resequence(server->items, item_count);
  server->item_count = item_count;
}

size_t mission_server_item_count(const mission_server_t *server) {
  return server != NULL ? server->item_count : 0;
}

void mission_server_close(mission_server_t *server) {
  if (server != NULL && server->subscription != NULL) {
    mavlink_message_subscription_cancel(server->subscription);
    server->subscription = NULL;
  }
}

void mission_server_destroy(mission_server_t *server) {
  if (server == NULL) {
    return;
  }
  mission_server_close(server);
  free(server);
}
