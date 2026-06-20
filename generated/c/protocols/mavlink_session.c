#include "mavlink_session.h"

#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "../mavlink_parser.h"

#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif

#define MAVLINK_IFLAG_SIGNED 0x01
#define MAVLINK_SIGNATURE_LENGTH 13

typedef struct {
  mavlink_frame_predicate_fn predicate;
  void *predicate_ctx;
  mavlink_cancellation_token_t *cancel;
  mavlink_frame_t *out_frame;
  int completed;
  mavlink_wait_result_t result;
} mavlink_pending_wait_t;

struct mavlink_message_subscription {
  mavlink_session_t *session;
  uint32_t message_id;
  uint8_t from_system_id;
  uint8_t from_component_id;
  mavlink_message_listener_fn on_data;
  void *user_data;
  int active;
};

struct mavlink_session {
  const mavlink_dialect_t *dialect;
  mavlink_link_t *link;
  uint8_t system_id;
  uint8_t component_id;
  mavlink_version_t version;
  mavlink_parser_t parser;
  int closed;
  uint8_t sequence;
  int signature_skip_remaining;

  mavlink_frame_t recent_frames[MAVLINK_SESSION_MAX_RECENT_FRAMES];
  int recent_count;

  mavlink_message_subscription_t subscriptions[MAVLINK_SESSION_MAX_SUBSCRIPTIONS];
  int subscription_count;

  mavlink_pending_wait_t *pending_wait;
};

static void mavlink_sleep_ms(int ms) {
  if (ms <= 0) {
    return;
  }
#ifdef _WIN32
  Sleep((DWORD)ms);
#else
  usleep((unsigned int)ms * 1000U);
#endif
}

static uint64_t mavlink_now_ms(void) {
#ifdef _WIN32
  return (uint64_t)GetTickCount64();
#else
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (uint64_t)ts.tv_sec * 1000ULL + (uint64_t)ts.tv_nsec / 1000000ULL;
#endif
}

static void mavlink_parser_reset_context(mavlink_parser_t *parser) {
  parser->state = MAVLINK_PARSER_INIT;
  parser->version = MAVLINK_VERSION_V1;
  parser->payload_length = 0;
  parser->incompatibility_flags = 0;
  parser->compatibility_flags = 0;
  parser->sequence = 0;
  parser->system_id = 0;
  parser->component_id = 0;
  parser->message_id_low = 0;
  parser->message_id_middle = 0;
  parser->message_id_high = 0;
  parser->message_id = 0;
  parser->payload_cursor = 0;
  parser->crc_low_byte = 0;
  parser->crc_high_byte = 0;
}

static bool mavlink_session_parse_message(
  const mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *out_message,
  size_t out_message_size
) {
  (void)out_message_size;
  if (session->dialect == NULL || session->dialect->parse == NULL || out_message == NULL) {
    return false;
  }
  return session->dialect->parse(
    session->dialect,
    frame->message_id,
    frame->payload,
    frame->payload_len,
    out_message
  );
}

static void mavlink_session_push_recent(mavlink_session_t *session, const mavlink_frame_t *frame) {
  if (session->recent_count < MAVLINK_SESSION_MAX_RECENT_FRAMES) {
    session->recent_frames[session->recent_count++] = *frame;
    return;
  }
  memmove(
    session->recent_frames,
    session->recent_frames + 1,
    (size_t)(MAVLINK_SESSION_MAX_RECENT_FRAMES - 1) * sizeof(mavlink_frame_t)
  );
  session->recent_frames[MAVLINK_SESSION_MAX_RECENT_FRAMES - 1] = *frame;
}

static bool mavlink_session_frame_matches_source(
  const mavlink_frame_t *frame,
  uint8_t from_system_id,
  uint8_t from_component_id
) {
  if (from_system_id != 0 && frame->system_id != from_system_id) {
    return false;
  }
  if (from_component_id != 0 && frame->component_id != from_component_id) {
    return false;
  }
  return true;
}

static void mavlink_session_dispatch_frame(mavlink_session_t *session, const mavlink_frame_t *frame) {
  mavlink_session_push_recent(session, frame);

  for (int i = 0; i < session->subscription_count; i++) {
    mavlink_message_subscription_t *sub = &session->subscriptions[i];
    if (!sub->active || sub->on_data == NULL) {
      continue;
    }
    if (sub->message_id != 0 && sub->message_id != frame->message_id) {
      continue;
    }
    if (!mavlink_session_frame_matches_source(frame, sub->from_system_id, sub->from_component_id)) {
      continue;
    }

    uint8_t parsed_buf[512];
    void *parsed = NULL;
    if (mavlink_session_parse_message(session, frame, parsed_buf, sizeof(parsed_buf))) {
      parsed = parsed_buf;
    }
    sub->on_data(session, frame, parsed, sub->user_data);
  }

  if (session->pending_wait != NULL && !session->pending_wait->completed) {
    mavlink_pending_wait_t *wait = session->pending_wait;
    if (wait->predicate != NULL && wait->predicate(frame, wait->predicate_ctx)) {
      if (wait->out_frame != NULL) {
        *wait->out_frame = *frame;
      }
      wait->result = MAVLINK_WAIT_OK;
      wait->completed = 1;
    }
  }
}

static bool mavlink_session_emit_frame(mavlink_session_t *session, mavlink_parser_t *parser) {
  if (!mavlink_parser_check_crc(parser)) {
    return false;
  }

  int crc_extra = parser->dialect->crc_extra(parser->dialect, parser->message_id);
  if (crc_extra < 0) {
    return false;
  }

  mavlink_frame_t frame;
  frame.version = parser->version;
  frame.sequence = parser->sequence;
  frame.system_id = parser->system_id;
  frame.component_id = parser->component_id;
  frame.message_id = parser->message_id;
  frame.payload_len = parser->payload_length;
  memset(frame.payload, 0, sizeof(frame.payload));
  if (parser->payload_length > 0) {
    memcpy(frame.payload, parser->payload, parser->payload_length);
  }
  frame.crc_extra = (uint8_t)crc_extra;

  mavlink_session_dispatch_frame(session, &frame);
  return true;
}

static void mavlink_session_feed_byte(mavlink_session_t *session, uint8_t byte) {
  mavlink_parser_t *parser = &session->parser;

  if (session->signature_skip_remaining > 0) {
    session->signature_skip_remaining--;
    if (session->signature_skip_remaining == 0) {
      mavlink_parser_reset_context(parser);
    }
    return;
  }

  switch (parser->state) {
  case MAVLINK_PARSER_INIT:
    if (byte == MAVLINK_STX_V1) {
      parser->version = MAVLINK_VERSION_V1;
      parser->state = MAVLINK_PARSER_WAIT_PAYLOAD_LENGTH;
    } else if (byte == MAVLINK_STX_V2) {
      parser->version = MAVLINK_VERSION_V2;
      parser->state = MAVLINK_PARSER_WAIT_PAYLOAD_LENGTH;
    }
    break;
  case MAVLINK_PARSER_WAIT_PAYLOAD_LENGTH:
    parser->payload_length = byte;
    parser->state = parser->version == MAVLINK_VERSION_V1
      ? MAVLINK_PARSER_WAIT_PACKET_SEQUENCE
      : MAVLINK_PARSER_WAIT_INCOMPATIBILITY_FLAGS;
    break;
  case MAVLINK_PARSER_WAIT_INCOMPATIBILITY_FLAGS:
    parser->incompatibility_flags = byte;
    parser->state = MAVLINK_PARSER_WAIT_COMPATIBILITY_FLAGS;
    break;
  case MAVLINK_PARSER_WAIT_COMPATIBILITY_FLAGS:
    parser->compatibility_flags = byte;
    parser->state = MAVLINK_PARSER_WAIT_PACKET_SEQUENCE;
    break;
  case MAVLINK_PARSER_WAIT_PACKET_SEQUENCE:
    parser->sequence = byte;
    parser->state = MAVLINK_PARSER_WAIT_SYSTEM_ID;
    break;
  case MAVLINK_PARSER_WAIT_SYSTEM_ID:
    parser->system_id = byte;
    parser->state = MAVLINK_PARSER_WAIT_COMPONENT_ID;
    break;
  case MAVLINK_PARSER_WAIT_COMPONENT_ID:
    parser->component_id = byte;
    parser->state = parser->version == MAVLINK_VERSION_V1
      ? MAVLINK_PARSER_WAIT_MESSAGE_ID_HIGH
      : MAVLINK_PARSER_WAIT_MESSAGE_ID_LOW;
    break;
  case MAVLINK_PARSER_WAIT_MESSAGE_ID_LOW:
    parser->message_id_low = byte;
    parser->state = MAVLINK_PARSER_WAIT_MESSAGE_ID_MIDDLE;
    break;
  case MAVLINK_PARSER_WAIT_MESSAGE_ID_MIDDLE:
    parser->message_id_middle = byte;
    parser->state = MAVLINK_PARSER_WAIT_MESSAGE_ID_HIGH;
    break;
  case MAVLINK_PARSER_WAIT_MESSAGE_ID_HIGH:
    if (parser->version == MAVLINK_VERSION_V1) {
      parser->message_id = byte;
    } else {
      parser->message_id_high = byte;
      parser->message_id =
        ((uint32_t)parser->message_id_high << 16) |
        ((uint32_t)parser->message_id_middle << 8) |
        (uint32_t)parser->message_id_low;
    }
    if (parser->payload_length == 0) {
      parser->state = MAVLINK_PARSER_WAIT_CRC_LOW_BYTE;
    } else {
      parser->payload_cursor = 0;
      parser->state = MAVLINK_PARSER_WAIT_PAYLOAD_END;
    }
    break;
  case MAVLINK_PARSER_WAIT_PAYLOAD_END:
    if (parser->payload_cursor < parser->payload_length) {
      parser->payload[parser->payload_cursor++] = byte;
    }
    if (parser->payload_cursor == parser->payload_length) {
      parser->state = MAVLINK_PARSER_WAIT_CRC_LOW_BYTE;
    }
    break;
  case MAVLINK_PARSER_WAIT_CRC_LOW_BYTE:
    parser->crc_low_byte = byte;
    parser->state = MAVLINK_PARSER_WAIT_CRC_HIGH_BYTE;
    break;
  case MAVLINK_PARSER_WAIT_CRC_HIGH_BYTE:
    parser->crc_high_byte = byte;
    if (parser->version == MAVLINK_VERSION_V2 &&
        (parser->incompatibility_flags & MAVLINK_IFLAG_SIGNED) != 0) {
      session->signature_skip_remaining = MAVLINK_SIGNATURE_LENGTH;
      mavlink_parser_reset_context(parser);
      break;
    }
    mavlink_session_emit_frame(session, parser);
    mavlink_parser_reset_context(parser);
    break;
  default:
    mavlink_parser_reset_context(parser);
    break;
  }
}

static void mavlink_session_on_receive(void *ctx, const uint8_t *data, size_t len) {
  mavlink_session_t *session = (mavlink_session_t *)ctx;
  if (session == NULL || session->closed) {
    return;
  }
  for (size_t i = 0; i < len; i++) {
    mavlink_session_feed_byte(session, data[i]);
  }
}

mavlink_session_t *mavlink_session_create(
  const mavlink_dialect_t *dialect,
  mavlink_link_t *link,
  uint8_t system_id,
  uint8_t component_id,
  mavlink_version_t version
) {
  mavlink_session_t *session = (mavlink_session_t *)calloc(1, sizeof(*session));
  if (session == NULL) {
    return NULL;
  }

  session->dialect = dialect;
  session->link = link;
  session->system_id = system_id;
  session->component_id = component_id;
  session->version = version;
  mavlink_parser_init(&session->parser, dialect);

  if (link != NULL && link->set_on_receive != NULL) {
    link->set_on_receive(link, mavlink_session_on_receive, session);
  }

  return session;
}

const mavlink_dialect_t *mavlink_session_dialect(const mavlink_session_t *session) {
  return session != NULL ? session->dialect : NULL;
}

uint8_t mavlink_session_system_id(const mavlink_session_t *session) {
  return session != NULL ? session->system_id : 0;
}

uint8_t mavlink_session_component_id(const mavlink_session_t *session) {
  return session != NULL ? session->component_id : 0;
}

int mavlink_session_send(
  mavlink_session_t *session,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t *payload,
  size_t payload_len
) {
  if (session == NULL || session->closed || session->link == NULL || session->link->send == NULL) {
    return -1;
  }

  mavlink_frame_t frame;
  mavlink_frame_init_v2(
    &frame,
    session->sequence++,
    session->system_id,
    session->component_id,
    message_id,
    crc_extra,
    payload,
    payload_len
  );

  uint8_t wire[MAVLINK_MAX_FRAME_SIZE];
  size_t wire_len = mavlink_frame_serialize_v2(&frame, wire, sizeof(wire));
  if (wire_len == 0) {
    return -1;
  }
  return session->link->send(session->link, wire, wire_len);
}

mavlink_message_subscription_t *mavlink_session_listen_message(
  mavlink_session_t *session,
  uint32_t message_id,
  uint8_t from_system_id,
  uint8_t from_component_id,
  mavlink_message_listener_fn on_data,
  void *user_data
) {
  if (session == NULL || session->subscription_count >= MAVLINK_SESSION_MAX_SUBSCRIPTIONS) {
    return NULL;
  }

  mavlink_message_subscription_t *sub = &session->subscriptions[session->subscription_count++];
  sub->session = session;
  sub->message_id = message_id;
  sub->from_system_id = from_system_id;
  sub->from_component_id = from_component_id;
  sub->on_data = on_data;
  sub->user_data = user_data;
  sub->active = 1;
  return sub;
}

void mavlink_message_subscription_cancel(mavlink_message_subscription_t *subscription) {
  if (subscription == NULL) {
    return;
  }
  subscription->active = 0;
  subscription->on_data = NULL;
}

bool mavlink_message_subscription_is_active(const mavlink_message_subscription_t *subscription) {
  return subscription != NULL && subscription->active != 0;
}

typedef struct {
  mavlink_frame_predicate_fn inner;
  void *inner_ctx;
  uint8_t from_system_id;
  uint8_t from_component_id;
} mavlink_wait_message_ctx_t;

static bool mavlink_wait_message_wrapper(const mavlink_frame_t *frame, void *user_data) {
  mavlink_wait_message_ctx_t *ctx = (mavlink_wait_message_ctx_t *)user_data;
  if (!mavlink_session_frame_matches_source(frame, ctx->from_system_id, ctx->from_component_id)) {
    return false;
  }
  return ctx->inner != NULL && ctx->inner(frame, ctx->inner_ctx);
}

static mavlink_wait_result_t mavlink_session_wait_for_frame_impl(
  mavlink_session_t *session,
  mavlink_frame_predicate_fn predicate,
  void *predicate_ctx,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_frame_t *out_frame
) {
  if (session == NULL || session->closed) {
    return MAVLINK_WAIT_CLOSED;
  }
  if (mavlink_cancellation_token_is_cancelled(cancel)) {
    return MAVLINK_WAIT_CANCELLED;
  }

  for (int i = 0; i < session->recent_count; i++) {
    const mavlink_frame_t *frame = &session->recent_frames[i];
    if (predicate != NULL && predicate(frame, predicate_ctx)) {
      if (out_frame != NULL) {
        *out_frame = *frame;
      }
      return MAVLINK_WAIT_OK;
    }
  }

  mavlink_pending_wait_t wait = {
    .predicate = predicate,
    .predicate_ctx = predicate_ctx,
    .cancel = cancel,
    .out_frame = out_frame,
    .completed = 0,
    .result = MAVLINK_WAIT_TIMEOUT,
  };
  session->pending_wait = &wait;

  uint64_t deadline = mavlink_now_ms() + (uint64_t)(timeout_ms > 0 ? timeout_ms : 5000);
  mavlink_wait_result_t result = MAVLINK_WAIT_TIMEOUT;

  while (!wait.completed) {
    if (session->closed) {
      result = MAVLINK_WAIT_CLOSED;
      break;
    }
    if (mavlink_cancellation_token_is_cancelled(cancel)) {
      result = MAVLINK_WAIT_CANCELLED;
      break;
    }
    if (mavlink_now_ms() >= deadline) {
      result = MAVLINK_WAIT_TIMEOUT;
      break;
    }
    mavlink_sleep_ms(1);
  }

  if (wait.completed) {
    result = wait.result;
  }

  session->pending_wait = NULL;
  return result;
}

mavlink_wait_result_t mavlink_session_wait_for_frame(
  mavlink_session_t *session,
  mavlink_frame_predicate_fn predicate,
  void *predicate_ctx,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_frame_t *out_frame
) {
  return mavlink_session_wait_for_frame_impl(
    session,
    predicate,
    predicate_ctx,
    timeout_ms,
    cancel,
    out_frame
  );
}

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
) {
  mavlink_wait_message_ctx_t ctx = {
    .inner = predicate,
    .inner_ctx = predicate_ctx,
    .from_system_id = from_system_id,
    .from_component_id = from_component_id,
  };

  mavlink_wait_result_t result = mavlink_session_wait_for_frame_impl(
    session,
    mavlink_wait_message_wrapper,
    &ctx,
    timeout_ms,
    cancel,
    out_frame
  );

  if (result == MAVLINK_WAIT_OK && out_frame != NULL && out_message != NULL) {
    if (!mavlink_session_parse_message(session, out_frame, out_message, out_message_size)) {
      return MAVLINK_WAIT_ERROR;
    }
  }
  return result;
}

typedef struct {
  uint32_t message_id;
} mavlink_wait_message_id_ctx_t;

static bool mavlink_wait_message_id_predicate(const mavlink_frame_t *frame, void *user_data) {
  mavlink_wait_message_id_ctx_t *ctx = (mavlink_wait_message_id_ctx_t *)user_data;
  return frame->message_id == ctx->message_id;
}

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
) {
  mavlink_wait_message_id_ctx_t ctx = { .message_id = message_id };
  return mavlink_session_wait_for_message(
    session,
    mavlink_wait_message_id_predicate,
    &ctx,
    from_system_id,
    from_component_id,
    timeout_ms,
    cancel,
    out_frame,
    out_message,
    out_message_size
  );
}

void mavlink_session_close(mavlink_session_t *session) {
  if (session == NULL || session->closed) {
    return;
  }
  session->closed = 1;
  if (session->link != NULL && session->link->close != NULL) {
    session->link->close(session->link);
  }
  free(session);
}
