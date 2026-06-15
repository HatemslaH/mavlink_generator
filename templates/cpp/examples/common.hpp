#pragma once

#include <cstdio>

#include "../mavlink.hpp"

/// Ground control station identity (MAVLink convention).
inline constexpr uint8_t GCS_SYSTEM_ID = 255;
inline constexpr uint8_t GCS_COMPONENT_ID = 190;

/// Simulated autopilot identity.
inline constexpr uint8_t DRONE_SYSTEM_ID = 1;
inline constexpr uint8_t DRONE_COMPONENT_ID = 1;

inline void mavlink_frame_from_gcs(
  mavlink::frame_t& frame,
  uint8_t sequence,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t* payload,
  size_t payload_len
) {
  mavlink::mavlink_frame_init_v2(
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

inline void mavlink_frame_from_drone(
  mavlink::frame_t& frame,
  uint8_t sequence,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t* payload,
  size_t payload_len
) {
  mavlink::mavlink_frame_init_v2(
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

inline void mavlink_param_id_from_string(char out[16], const char* name) {
  mavlink::mavlink_memset_s(out, 16, 0, 16);
  mavlink::mavlink_strncpy_s(out, 16, name, 15);
}

inline void mavlink_param_id_to_string(const char id[16], char* out, size_t out_len) {
  size_t end = 0;
  while (end < 16 && id[end] != '\0') {
    end++;
  }
  if (out_len == 0) {
    return;
  }
  size_t copy_len = end < out_len - 1 ? end : out_len - 1;
  mavlink::mavlink_memcpy_s(out, out_len, id, copy_len);
  out[copy_len] = '\0';
}

inline void mavlink_log_frame(
  const char* direction,
  uint32_t message_id,
  uint8_t system_id,
  uint8_t component_id
) {
  std::printf(
    "%s msgId=%u sys=%u comp=%u\n",
    direction,
    message_id,
    system_id,
    component_id
  );
}
