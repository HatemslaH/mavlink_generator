#ifndef MAVLINK_PROTOCOLS_PARAMETER_PROTOCOL_H
#define MAVLINK_PROTOCOLS_PARAMETER_PROTOCOL_H

#include <stddef.h>
#include <stdint.h>

#include "mavlink_cancellation.h"
#include "mavlink_session.h"
#include "param_codec.h"

#define MAVLINK_PARAM_CACHE_MAX 128
/// Max parameter index tracked during fetch (ArduPilot stacks often exceed 1000).
#define MAVLINK_PARAM_INDEX_MAX 2048
#define MAVLINK_PARAM_INBOX_MAX 2048

typedef struct param_entry {
  char id[17];
  double value;
  mavlink_param_type_t type;
  uint16_t index;
  uint16_t count;
} param_entry_t;

typedef void (*param_progress_callback_fn)(const param_entry_t *entry, int received, int expected, void *user_data);

typedef struct parameter_protocol parameter_protocol_t;
typedef struct parameter_server parameter_server_t;

parameter_protocol_t *parameter_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int idle_timeout_ms,
  int request_timeout_ms
);

void parameter_protocol_clear_cache(parameter_protocol_t *protocol);
mavlink_param_type_t parameter_protocol_type_for_name(const parameter_protocol_t *protocol, const char *name);

mavlink_wait_result_t parameter_protocol_fetch_all(
  parameter_protocol_t *protocol,
  param_entry_t *out_entries,
  size_t max_entries,
  size_t *out_count,
  param_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t parameter_protocol_read_by_name(
  parameter_protocol_t *protocol,
  const char *name,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t parameter_protocol_read_by_index(
  parameter_protocol_t *protocol,
  int16_t index,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t parameter_protocol_write(
  parameter_protocol_t *protocol,
  const char *name,
  double value,
  mavlink_param_type_t type,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t parameter_protocol_write_by_name(
  parameter_protocol_t *protocol,
  const char *name,
  double value,
  mavlink_param_type_t type,
  param_entry_t *out_entry,
  mavlink_cancellation_token_t *cancel
);

void parameter_protocol_destroy(parameter_protocol_t *protocol);

parameter_server_t *parameter_server_create(mavlink_session_t *session);
void parameter_server_set(parameter_server_t *server, const char *name, double value, mavlink_param_type_t type);
void parameter_server_close(parameter_server_t *server);
void parameter_server_destroy(parameter_server_t *server);

#endif
