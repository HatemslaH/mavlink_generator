#ifndef MAVLINK_DIALECT_H
#define MAVLINK_DIALECT_H

#include "types.h"

typedef struct mavlink_dialect mavlink_dialect_t;

struct mavlink_dialect {
  int version;
  bool (*parse)(
    const mavlink_dialect_t *dialect,
    uint32_t message_id,
    const uint8_t *payload,
    size_t payload_len,
    void *out_message
  );
  int (*crc_extra)(const mavlink_dialect_t *dialect, uint32_t message_id);
};

#endif
