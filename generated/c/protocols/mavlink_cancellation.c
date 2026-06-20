#include "mavlink_cancellation.h"

#include <stdlib.h>

struct mavlink_cancellation_token {
  int cancelled;
};

mavlink_cancellation_token_t *mavlink_cancellation_token_create(void) {
  mavlink_cancellation_token_t *token =
    (mavlink_cancellation_token_t *)calloc(1, sizeof(*token));
  return token;
}

void mavlink_cancellation_token_cancel(mavlink_cancellation_token_t *token) {
  if (token == NULL) {
    return;
  }
  token->cancelled = 1;
}

bool mavlink_cancellation_token_is_cancelled(const mavlink_cancellation_token_t *token) {
  return token != NULL && token->cancelled != 0;
}

int mavlink_cancellation_token_throw_if_cancelled(const mavlink_cancellation_token_t *token) {
  if (mavlink_cancellation_token_is_cancelled(token)) {
    return -1;
  }
  return 0;
}

void mavlink_cancellation_token_dispose(mavlink_cancellation_token_t *token) {
  free(token);
}
