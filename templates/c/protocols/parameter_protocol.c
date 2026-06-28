#include "parameter_protocol.h"

#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#include "../mavlink.h"

struct parameter_protocol {
  mavlink_session_t *session;
  uint8_t target_system;
  uint8_t target_component;
  int idle_timeout_ms;
  int request_timeout_ms;
  param_entry_t cache[MAVLINK_PARAM_CACHE_MAX];
  int cache_count;
};

typedef struct {
  char name[17];
  double value;
  mavlink_param_type_t type;
} parameter_server_value_t;

struct parameter_server {
  mavlink_session_t *session;
  parameter_server_value_t values[MAVLINK_PARAM_CACHE_MAX];
  int value_count;
  mavlink_message_subscription_t *subscription;
};

static void parameter_protocol_remember(parameter_protocol_t *protocol, const param_entry_t *entry) {
  for (int i = 0; i < protocol->cache_count; i++) {
    if (strcmp(protocol->cache[i].id, entry->id) == 0) {
      protocol->cache[i] = *entry;
      return;
    }
  }
  if (protocol->cache_count < MAVLINK_PARAM_CACHE_MAX) {
    protocol->cache[protocol->cache_count++] = *entry;
  }
}

static param_entry_t parameter_entry_from_value(const param_value_t *value) {
  param_entry_t entry;
  mavlink_param_codec_param_id_to_string(value->param_id, entry.id, sizeof(entry.id));
  entry.value = mavlink_param_codec_decode_value(value->param_value, value->param_type);
  entry.type = value->param_type;
  entry.index = value->param_index;
  entry.count = value->param_count;
  return entry;
}

typedef struct {
  param_value_t items[MAVLINK_PARAM_INBOX_MAX];
  int count;
} param_fetch_inbox_t;

typedef struct {
  bool *seen;
} param_unseen_wait_ctx_t;

static void param_fetch_inbox_on_message(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  (void)frame;
  param_fetch_inbox_t *inbox = (param_fetch_inbox_t *)user_data;
  if (parsed_message == NULL) {
    return;
  }
  if (inbox->count >= MAVLINK_PARAM_INBOX_MAX) {
    memmove(
      inbox->items,
      inbox->items + 1,
      (size_t)(MAVLINK_PARAM_INBOX_MAX - 1) * sizeof(param_value_t)
    );
    inbox->count = MAVLINK_PARAM_INBOX_MAX - 1;
  }
  inbox->items[inbox->count++] = *(const param_value_t *)parsed_message;
}

static int param_fetch_take_next(
  param_fetch_inbox_t *inbox,
  const bool *seen,
  param_value_t *out
) {
  int i = 0;
  while (i < inbox->count) {
    const uint16_t idx = inbox->items[i].param_index;
    if (idx >= MAVLINK_PARAM_INDEX_MAX || seen[idx]) {
      if (i < inbox->count - 1) {
        memmove(
          &inbox->items[i],
          &inbox->items[i + 1],
          (size_t)(inbox->count - i - 1) * sizeof(param_value_t)
        );
      }
      inbox->count--;
      continue;
    }
    *out = inbox->items[i];
    if (i < inbox->count - 1) {
      memmove(
        &inbox->items[i],
        &inbox->items[i + 1],
        (size_t)(inbox->count - i - 1) * sizeof(param_value_t)
      );
    }
    inbox->count--;
    return 1;
  }
  return 0;
}

static bool param_fetch_unseen_predicate(const mavlink_frame_t *frame, void *user_data) {
  param_unseen_wait_ctx_t *ctx = (param_unseen_wait_ctx_t *)user_data;
  if (frame->message_id != param_value_MSG_ID) {
    return false;
  }
  param_value_t value;
  param_value_parse(frame->payload, &value);
  if (value.param_index >= MAVLINK_PARAM_INDEX_MAX || ctx->seen[value.param_index]) {
    return false;
  }
  return true;
}

static int param_fetch_find_missing_index(const bool *seen, int expected_count) {
  for (int i = 0; i < expected_count && i < MAVLINK_PARAM_INDEX_MAX; i++) {
    if (!seen[i]) {
      return i;
    }
  }
  return -1;
}

static mavlink_wait_result_t param_fetch_wait_for_next(
  parameter_protocol_t *protocol,
  param_fetch_inbox_t *inbox,
  bool *seen,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  param_value_t *out_value
) {
  if (param_fetch_take_next(inbox, seen, out_value)) {
    return MAVLINK_WAIT_OK;
  }

  param_unseen_wait_ctx_t ctx = { .seen = seen };
  mavlink_frame_t frame;
  mavlink_wait_result_t wait = mavlink_session_wait_for_message(
    protocol->session,
    param_fetch_unseen_predicate,
    &ctx,
    protocol->target_system,
    protocol->target_component,
    timeout_ms,
    cancel,
    &frame,
    out_value,
    sizeof(*out_value)
  );
  if (wait == MAVLINK_WAIT_OK) {
    return MAVLINK_WAIT_OK;
  }
  return wait;
}

parameter_protocol_t *parameter_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int idle_timeout_ms,
  int request_timeout_ms
) {
  parameter_protocol_t *protocol = (parameter_protocol_t *)calloc(1, sizeof(*protocol));
  if (protocol == NULL) {
    return NULL;
  }
  protocol->session = session;
  protocol->target_system = target_system;
  protocol->target_component = target_component;
  protocol->idle_timeout_ms = idle_timeout_ms > 0 ? idle_timeout_ms : 500;
  protocol->request_timeout_ms = request_timeout_ms > 0 ? request_timeout_ms : 3000;
  return protocol;
}

void parameter_protocol_clear_cache(parameter_protocol_t *protocol) {
  if (protocol != NULL) {
    protocol->cache_count = 0;
  }
}

mavlink_param_type_t parameter_protocol_type_for_name(const parameter_protocol_t *protocol, const char *name) {
  if (protocol == NULL || name == NULL) {
    return MAVLINK_PARAM_TYPE_REAL32;
  }
  for (int i = 0; i < protocol->cache_count; i++) {
    if (strcmp(protocol->cache[i].id, name) == 0) {
      return protocol->cache[i].type;
    }
  }
  return MAVLINK_PARAM_TYPE_REAL32;
}

mavlink_wait_result_t parameter_protocol_fetch_all(
  parameter_protocol_t *protocol,
  param_entry_t *out_entries,
  size_t max_entries,
  size_t *out_count,
  param_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel
) {
  if (protocol == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  if (mavlink_cancellation_token_throw_if_cancelled(cancel) != 0) {
    return MAVLINK_WAIT_CANCELLED;
  }

  param_fetch_inbox_t inbox = { .count = 0 };
  mavlink_message_subscription_t *subscription = mavlink_session_listen_message(
    protocol->session,
    param_value_MSG_ID,
    protocol->target_system,
    protocol->target_component,
    param_fetch_inbox_on_message,
    &inbox
  );
  if (subscription == NULL) {
    return MAVLINK_WAIT_ERROR;
  }

  param_request_list_t request = {
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
  };
  uint8_t payload[param_request_list_ENCODED_LENGTH];
  param_request_list_serialize(&request, payload);
  if (mavlink_session_send(
        protocol->session,
        param_request_list_MSG_ID,
        param_request_list_CRC_EXTRA,
        payload,
        param_request_list_ENCODED_LENGTH) != 0) {
    mavlink_message_subscription_cancel(subscription);
    return MAVLINK_WAIT_ERROR;
  }

  int expected_count = -1;
  bool seen[MAVLINK_PARAM_INDEX_MAX] = {false};
  uint8_t retry_counts[MAVLINK_PARAM_INDEX_MAX] = {0};
  int seen_count = 0;
  int is_retrying = 0;
  size_t received = 0;

  while (true) {
    if (mavlink_cancellation_token_throw_if_cancelled(cancel) != 0) {
      mavlink_message_subscription_cancel(subscription);
      return MAVLINK_WAIT_CANCELLED;
    }

    param_value_t value;
    int have_value = param_fetch_take_next(&inbox, seen, &value);
    if (!have_value) {
      const int timeout =
        expected_count < 0 || is_retrying != 0
          ? protocol->request_timeout_ms
          : protocol->idle_timeout_ms;

      mavlink_wait_result_t wait = param_fetch_wait_for_next(
        protocol,
        &inbox,
        seen,
        timeout,
        cancel,
        &value
      );
      if (wait != MAVLINK_WAIT_OK) {
        if (wait == MAVLINK_WAIT_TIMEOUT && expected_count >= 0) {
          const int missing_index = param_fetch_find_missing_index(seen, expected_count);
          if (missing_index < 0) {
            break;
          }

          if (missing_index < MAVLINK_PARAM_INDEX_MAX &&
              retry_counts[missing_index] >= 3) {
            mavlink_message_subscription_cancel(subscription);
            return MAVLINK_WAIT_TIMEOUT;
          }

          if (missing_index < MAVLINK_PARAM_INDEX_MAX) {
            retry_counts[missing_index]++;
          }
          is_retrying = 1;

          param_request_read_t read_request = {0};
          read_request.param_index = (int16_t)missing_index;
          read_request.target_system = protocol->target_system;
          read_request.target_component = protocol->target_component;

          uint8_t req_payload[param_request_read_ENCODED_LENGTH];
          param_request_read_serialize(&read_request, req_payload);
          mavlink_session_send(
            protocol->session,
            param_request_read_MSG_ID,
            param_request_read_CRC_EXTRA,
            req_payload,
            param_request_read_ENCODED_LENGTH
          );
          continue;
        }

        mavlink_message_subscription_cancel(subscription);
        return wait;
      }
      is_retrying = 0;
    } else {
      is_retrying = 0;
    }

    const uint16_t idx = value.param_index;
    if (idx >= MAVLINK_PARAM_INDEX_MAX || seen[idx]) {
      continue;
    }

    seen[idx] = true;
    seen_count++;

    if (expected_count < 0) {
      expected_count = (int)value.param_count;
    }

    param_entry_t entry = parameter_entry_from_value(&value);
    parameter_protocol_remember(protocol, &entry);
    if (out_entries != NULL && received < max_entries) {
      out_entries[received] = entry;
    }
    received++;
    if (on_progress != NULL) {
      on_progress(&entry, seen_count, expected_count, progress_ctx);
    }

    if (expected_count >= 0 && seen_count >= expected_count) {
      break;
    }
  }

  mavlink_message_subscription_cancel(subscription);

  if (out_count != NULL) {
    *out_count = received;
  }
  return MAVLINK_WAIT_OK;
}

static mavlink_wait_result_t parameter_protocol_read_impl(
  parameter_protocol_t *protocol,
  const char *param_id,
  int16_t param_index,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
) {
  param_request_read_t request = {
    .param_index = param_index,
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
  };
  if (param_id != NULL) {
    mavlink_param_codec_param_id_from_string(request.param_id, param_id);
  }

  uint8_t payload[param_request_read_ENCODED_LENGTH];
  param_request_read_serialize(&request, payload);
  if (mavlink_session_send(
        protocol->session,
        param_request_read_MSG_ID,
        param_request_read_CRC_EXTRA,
        payload,
        param_request_read_ENCODED_LENGTH) != 0) {
    return MAVLINK_WAIT_ERROR;
  }

  mavlink_frame_t frame;
  param_value_t value;
  mavlink_wait_result_t wait = mavlink_session_wait_for_message_id(
    protocol->session,
    param_value_MSG_ID,
    protocol->target_system,
    protocol->target_component,
    protocol->request_timeout_ms,
    cancel,
    &frame,
    &value,
    sizeof(value)
  );
  if (wait != MAVLINK_WAIT_OK) {
    return wait;
  }

  param_entry_t entry = parameter_entry_from_value(&value);
  parameter_protocol_remember(protocol, &entry);
  if (out_entry != NULL) {
    *out_entry = entry;
  }
  return MAVLINK_WAIT_OK;
}

mavlink_wait_result_t parameter_protocol_read_by_name(
  parameter_protocol_t *protocol,
  const char *name,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
) {
  if (protocol == NULL || name == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  return parameter_protocol_read_impl(protocol, name, -1, out_entry, cancel);
}

mavlink_wait_result_t parameter_protocol_read_by_index(
  parameter_protocol_t *protocol,
  int16_t index,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
) {
  if (protocol == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  return parameter_protocol_read_impl(protocol, NULL, index, out_entry, cancel);
}

mavlink_wait_result_t parameter_protocol_write(
  parameter_protocol_t *protocol,
  const char *name,
  double value,
  mavlink_param_type_t type,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
) {
  if (protocol == NULL || name == NULL) {
    return MAVLINK_WAIT_ERROR;
  }

  param_set_t set = {
    .param_value = mavlink_param_codec_encode_value(value, type),
    .target_system = protocol->target_system,
    .target_component = protocol->target_component,
    .param_type = type,
  };
  mavlink_param_codec_param_id_from_string(set.param_id, name);

  uint8_t payload[param_set_ENCODED_LENGTH];
  param_set_serialize(&set, payload);
  if (mavlink_session_send(
        protocol->session,
        param_set_MSG_ID,
        param_set_CRC_EXTRA,
        payload,
        param_set_ENCODED_LENGTH) != 0) {
    return MAVLINK_WAIT_ERROR;
  }

  typedef struct {
    const char *name;
  } write_ctx_t;
  (void)sizeof(write_ctx_t);

  mavlink_frame_t frame;
  param_value_t ack;

  mavlink_wait_result_t wait = mavlink_session_wait_for_message_id(
    protocol->session,
    param_value_MSG_ID,
    protocol->target_system,
    protocol->target_component,
    protocol->request_timeout_ms,
    cancel,
    &frame,
    &ack,
    sizeof(ack)
  );
  if (wait != MAVLINK_WAIT_OK) {
    return wait;
  }

  char ack_id[17];
  mavlink_param_codec_param_id_to_string(ack.param_id, ack_id, sizeof(ack_id));
  if (strcmp(ack_id, name) != 0) {
    return MAVLINK_WAIT_ERROR;
  }

  param_entry_t entry = parameter_entry_from_value(&ack);
  parameter_protocol_remember(protocol, &entry);
  if (out_entry != NULL) {
    *out_entry = entry;
  }
  return MAVLINK_WAIT_OK;
}

mavlink_wait_result_t parameter_protocol_write_by_name(
  parameter_protocol_t *protocol,
  const char *name,
  double value,
  mavlink_param_type_t type,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
) {
  mavlink_param_type_t resolved = type;
  if (resolved == 0) {
    resolved = parameter_protocol_type_for_name(protocol, name);
  }
  return parameter_protocol_write(protocol, name, value, resolved, out_entry, cancel);
}

void parameter_protocol_destroy(parameter_protocol_t *protocol) {
  free(protocol);
}

static int parameter_server_find_index(parameter_server_t *server, const char *name) {
  for (int i = 0; i < server->value_count; i++) {
    if (strcmp(server->values[i].name, name) == 0) {
      return i;
    }
  }
  return -1;
}

static void parameter_server_send_value(
  parameter_server_t *server,
  const char *name,
  const parameter_server_value_t *entry,
  int index,
  uint8_t target_system,
  uint8_t target_component
) {
  param_value_t value = {
    .param_value = mavlink_param_codec_encode_value(entry->value, entry->type),
    .param_count = (uint16_t)server->value_count,
    .param_index = (uint16_t)index,
    .param_type = entry->type,
  };
  mavlink_param_codec_param_id_from_string(value.param_id, name);
  uint8_t payload[param_value_ENCODED_LENGTH];
  param_value_serialize(&value, payload);
  mavlink_session_send(
    server->session,
    param_value_MSG_ID,
    param_value_CRC_EXTRA,
    payload,
    param_value_ENCODED_LENGTH
  );
  (void)target_system;
  (void)target_component;
}

static void parameter_server_on_frame(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  parameter_server_t *server = (parameter_server_t *)user_data;
  if (server == NULL || parsed_message == NULL) {
    return;
  }

  if (frame->message_id == param_request_list_MSG_ID) {
    const param_request_list_t *request = (const param_request_list_t *)parsed_message;
    if (request->target_system != mavlink_session_system_id(server->session) && request->target_system != 0) {
      return;
    }
    for (int i = 0; i < server->value_count; i++) {
      parameter_server_send_value(
        server,
        server->values[i].name,
        &server->values[i],
        i,
        frame->system_id,
        frame->component_id
      );
    }
    return;
  }

  if (frame->message_id == param_request_read_MSG_ID) {
    const param_request_read_t *request = (const param_request_read_t *)parsed_message;
    if (request->target_system != mavlink_session_system_id(server->session) && request->target_system != 0) {
      return;
    }
    const char *name = NULL;
    char name_buf[17];
    if (request->param_index >= 0 && request->param_index < server->value_count) {
      name = server->values[request->param_index].name;
    } else {
      mavlink_param_codec_param_id_to_string(request->param_id, name_buf, sizeof(name_buf));
      name = name_buf;
    }
    int index = parameter_server_find_index(server, name);
    if (index >= 0) {
      parameter_server_send_value(
        server,
        server->values[index].name,
        &server->values[index],
        index,
        frame->system_id,
        frame->component_id
      );
    }
    return;
  }

  if (frame->message_id == param_set_MSG_ID) {
    const param_set_t *set = (const param_set_t *)parsed_message;
    if (set->target_system != mavlink_session_system_id(server->session)) {
      return;
    }
    char name_buf[17];
    mavlink_param_codec_param_id_to_string(set->param_id, name_buf, sizeof(name_buf));
    int index = parameter_server_find_index(server, name_buf);
    parameter_server_value_t entry = {
      .value = mavlink_param_codec_decode_value(set->param_value, set->param_type),
      .type = set->param_type,
    };
    mavlink_strncpy_s(entry.name, sizeof(entry.name), name_buf, sizeof(entry.name) - 1);
    if (index < 0 && server->value_count < MAVLINK_PARAM_CACHE_MAX) {
      index = server->value_count++;
      server->values[index] = entry;
    } else if (index >= 0) {
      server->values[index] = entry;
    }
    if (index >= 0) {
      parameter_server_send_value(
        server,
        server->values[index].name,
        &server->values[index],
        index,
        frame->system_id,
        frame->component_id
      );
    }
  }
}

parameter_server_t *parameter_server_create(mavlink_session_t *session) {
  parameter_server_t *server = (parameter_server_t *)calloc(1, sizeof(*server));
  if (server == NULL) {
    return NULL;
  }
  server->session = session;
  server->subscription = mavlink_session_listen_message(server->session, 0, 0, 0, parameter_server_on_frame, server);
  return server;
}

void parameter_server_set(parameter_server_t *server, const char *name, double value, mavlink_param_type_t type) {
  if (server == NULL || name == NULL) {
    return;
  }
  int index = parameter_server_find_index(server, name);
  if (index < 0 && server->value_count < MAVLINK_PARAM_CACHE_MAX) {
    index = server->value_count++;
  }
  if (index < 0) {
    return;
  }
  mavlink_strncpy_s(server->values[index].name, sizeof(server->values[index].name), name, 16);
  server->values[index].value = value;
  server->values[index].type = type;
}

void parameter_server_close(parameter_server_t *server) {
  if (server == NULL) {
    return;
  }
  if (server->subscription != NULL) {
    mavlink_message_subscription_cancel(server->subscription);
    server->subscription = NULL;
  }
}

void parameter_server_destroy(parameter_server_t *server) {
  if (server == NULL) {
    return;
  }
  parameter_server_close(server);
  free(server);
}
