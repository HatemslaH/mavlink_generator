#ifndef MAVLINK_EXAMPLES_COMMON_H
#define MAVLINK_EXAMPLES_COMMON_H

#include <stdio.h>

#include "../mavlink.h"

/// Ground control station identity (MAVLink convention).
#define GCS_SYSTEM_ID 255
#define GCS_COMPONENT_ID 190

/// Simulated autopilot identity.
#define DRONE_SYSTEM_ID 1
#define DRONE_COMPONENT_ID 1

static inline void mavlink_frame_from_gcs(
  mavlink_frame_t *frame,
  uint8_t sequence,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t *payload,
  size_t payload_len
) {
  mavlink_frame_init_v2(
    frame,
    sequence,
    GCS_SYSTEM_ID,
    GCS_COMPONENT_ID,
    message_id,
    crc_extra,
    payload,
    payload_len
  );
}

static inline void mavlink_frame_from_drone(
  mavlink_frame_t *frame,
  uint8_t sequence,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t *payload,
  size_t payload_len
) {
  mavlink_frame_init_v2(
    frame,
    sequence,
    DRONE_SYSTEM_ID,
    DRONE_COMPONENT_ID,
    message_id,
    crc_extra,
    payload,
    payload_len
  );
}

static inline void mavlink_param_id_from_string(char out[16], const char *name) {
  mavlink_memset_s(out, 16, 0, 16);
  mavlink_strncpy_s(out, 16, name, 15);
}

static inline void mavlink_param_id_to_string(const char id[16], char *out, size_t out_len) {
  size_t end = 0;
  while (end < 16 && id[end] != '\0') {
    end++;
  }
  if (out_len == 0) {
    return;
  }
  size_t copy_len = end < out_len - 1 ? end : out_len - 1;
  mavlink_memcpy_s(out, out_len, id, copy_len);
  out[copy_len] = '\0';
}

static inline void mavlink_log_frame(
  const char *direction,
  uint32_t message_id,
  uint8_t system_id,
  uint8_t component_id
) {
  printf(
    "%s msgId=%u sys=%u comp=%u\n",
    direction,
    message_id,
    system_id,
    component_id
  );
}

#endif
