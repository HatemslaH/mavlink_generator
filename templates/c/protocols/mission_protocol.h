#ifndef MAVLINK_PROTOCOLS_MISSION_PROTOCOL_H
#define MAVLINK_PROTOCOLS_MISSION_PROTOCOL_H

#include <stddef.h>
#include <stdint.h>

#include "../mavlink.h"
#include "command_protocol.h"
#include "mavlink_cancellation.h"
#include "mavlink_session.h"

#define MAVLINK_MISSION_MAX_ITEMS 128

typedef void (*mission_upload_progress_callback_fn)(
  int sent,
  int total,
  const mission_item_int_t *item,
  void *user_data
);

typedef void (*mission_download_progress_callback_fn)(
  int received,
  int total,
  const mission_item_int_t *item,
  void *user_data
);

typedef struct mission_set_current_result {
  uint16_t sequence;
  int has_command_ack;
  command_ack_t command_ack;
} mission_set_current_result_t;

typedef struct mission_protocol mission_protocol_t;
typedef struct mission_server mission_server_t;

mission_protocol_t *mission_protocol_create(
  mavlink_session_t *session,
  uint8_t target_system,
  uint8_t target_component,
  int item_timeout_ms,
  int operation_timeout_ms
);

mavlink_wait_result_t mission_protocol_upload(
  mission_protocol_t *protocol,
  const mission_item_int_t *items,
  size_t item_count,
  MAV_MISSION_TYPE mission_type,
  mission_upload_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel,
  MAV_MISSION_RESULT *out_result
);

mavlink_wait_result_t mission_protocol_download(
  mission_protocol_t *protocol,
  mission_item_int_t *out_items,
  size_t max_items,
  size_t *out_count,
  MAV_MISSION_TYPE mission_type,
  mission_download_progress_callback_fn on_progress,
  void *progress_ctx,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t mission_protocol_clear(
  mission_protocol_t *protocol,
  MAV_MISSION_TYPE mission_type,
  mavlink_cancellation_token_t *cancel,
  MAV_MISSION_RESULT *out_result
);

mavlink_wait_result_t mission_protocol_set_current(
  mission_protocol_t *protocol,
  uint16_t seq,
  mavlink_cancellation_token_t *cancel
);

mavlink_wait_result_t mission_protocol_set_current_with_command(
  mission_protocol_t *protocol,
  uint16_t seq,
  command_protocol_t *command,
  int also_send_command,
  int reset_mission,
  mavlink_cancellation_token_t *cancel,
  mission_set_current_result_t *out_result
);

void mission_protocol_destroy(mission_protocol_t *protocol);

mission_server_t *mission_server_create(mavlink_session_t *session, MAV_MISSION_TYPE mission_type);
void mission_server_replace_mission(mission_server_t *server, const mission_item_int_t *items, size_t item_count);
size_t mission_server_item_count(const mission_server_t *server);
void mission_server_close(mission_server_t *server);
void mission_server_destroy(mission_server_t *server);

#endif
