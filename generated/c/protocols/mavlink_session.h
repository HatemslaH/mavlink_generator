#ifndef MAVLINK_PROTOCOLS_MAVLINK_SESSION_H
#define MAVLINK_PROTOCOLS_MAVLINK_SESSION_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "../mavlink_dialect.h"
#include "../mavlink_frame.h"
#include "../mavlink_version.h"
#include "mavlink_cancellation.h"
#include "mavlink_link.h"

#define MAVLINK_SESSION_MAX_RECENT_FRAMES 64
#define MAVLINK_SESSION_MAX_SUBSCRIPTIONS 32

typedef struct mavlink_session mavlink_session_t;

typedef struct mavlink_message_subscription mavlink_message_subscription_t;

typedef enum {
  MAVLINK_WAIT_OK = 0,
  MAVLINK_WAIT_TIMEOUT = -1,
  MAVLINK_WAIT_CANCELLED = -2,
  MAVLINK_WAIT_CLOSED = -3,
  MAVLINK_WAIT_ERROR = -4,
} mavlink_wait_result_t;

typedef bool (*mavlink_frame_predicate_fn)(const mavlink_frame_t *frame, void *user_data);

typedef void (*mavlink_message_listener_fn)(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
);

mavlink_session_t *mavlink_session_create(
  const mavlink_dialect_t *dialect,
  mavlink_link_t *link,
  uint8_t system_id,
  uint8_t component_id,
  mavlink_version_t version
);

const mavlink_dialect_t *mavlink_session_dialect(const mavlink_session_t *session);

uint8_t mavlink_session_system_id(const mavlink_session_t *session);

uint8_t mavlink_session_component_id(const mavlink_session_t *session);

/// Serialize and send a typed message (caller supplies message id, crc, payload).
int mavlink_session_send(
  mavlink_session_t *session,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t *payload,
  size_t payload_len
);

/// Register a callback for frames matching message_id (0 = any) and optional source filters (0 = any).
mavlink_message_subscription_t *mavlink_session_listen_message(
  mavlink_session_t *session,
  uint32_t message_id,
  uint8_t from_system_id,
  uint8_t from_component_id,
  mavlink_message_listener_fn on_data,
  void *user_data
);

void mavlink_message_subscription_cancel(mavlink_message_subscription_t *subscription);

bool mavlink_message_subscription_is_active(const mavlink_message_subscription_t *subscription);

/// Blocking wait for the first frame matching predicate.
mavlink_wait_result_t mavlink_session_wait_for_frame(
  mavlink_session_t *session,
  mavlink_frame_predicate_fn predicate,
  void *predicate_ctx,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_frame_t *out_frame
);

/// Blocking wait for the first message matching predicate and optional source filters.
mavlink_wait_result_t mavlink_session_wait_for_message(
  mavlink_session_t *session,
  mavlink_frame_predicate_fn predicate,
  void *predicate_ctx,
  uint8_t from_system_id,
  uint8_t from_component_id,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_frame_t *out_frame,
  void *out_message,
  size_t out_message_size
);

/// Blocking wait for the first message with the given id.
mavlink_wait_result_t mavlink_session_wait_for_message_id(
  mavlink_session_t *session,
  uint32_t message_id,
  uint8_t from_system_id,
  uint8_t from_component_id,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_frame_t *out_frame,
  void *out_message,
  size_t out_message_size
);

void mavlink_session_close(mavlink_session_t *session);

#endif
