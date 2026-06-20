#ifndef MAVLINK_PROTOCOLS_MAVLINK_CANCELLATION_H
#define MAVLINK_PROTOCOLS_MAVLINK_CANCELLATION_H

#include <stdbool.h>

typedef struct mavlink_cancellation_token mavlink_cancellation_token_t;

/// Cooperative cancellation token for session waits and protocol flows.
mavlink_cancellation_token_t *mavlink_cancellation_token_create(void);

void mavlink_cancellation_token_cancel(mavlink_cancellation_token_t *token);

bool mavlink_cancellation_token_is_cancelled(const mavlink_cancellation_token_t *token);

/// Returns non-zero when cancelled (for early exit in protocol loops).
int mavlink_cancellation_token_throw_if_cancelled(const mavlink_cancellation_token_t *token);

void mavlink_cancellation_token_dispose(mavlink_cancellation_token_t *token);

#endif
