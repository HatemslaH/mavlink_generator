#pragma once

#include "crc.hpp"
#include "mavlink_memory.hpp"
#include "mavlink_version.hpp"
#include "types.hpp"

namespace mavlink {

inline constexpr uint8_t MAVLINK_STX_V1 = 0xFE;
inline constexpr uint8_t MAVLINK_STX_V2 = 0xFD;
inline constexpr size_t MAVLINK_MAX_FRAME_SIZE = 12 + 255 + 2;

struct frame_t {
  version_t version;
  uint8_t sequence;
  uint8_t system_id;
  uint8_t component_id;
  uint32_t message_id;
  uint8_t payload[255];
  size_t payload_len;
  uint8_t crc_extra;
};

inline size_t mavlink_trim_trailing_zeros(const uint8_t* payload, size_t payload_len) {
  while (payload_len > 0 && payload[payload_len - 1] == 0) {
    payload_len--;
  }
  return payload_len;
}

inline size_t mavlink_frame_serialize_v2(
  const frame_t& frame,
  uint8_t* out,
  size_t out_capacity
) {
  size_t payload_len = mavlink_trim_trailing_zeros(frame.payload, frame.payload_len);
  size_t frame_len = 12 + payload_len;
  if (out_capacity < frame_len) {
    return 0;
  }

  out[0] = MAVLINK_STX_V2;
  out[1] = static_cast<uint8_t>(payload_len);
  out[2] = 0;
  out[3] = 0;
  out[4] = frame.sequence;
  out[5] = frame.system_id;
  out[6] = frame.component_id;
  out[7] = static_cast<uint8_t>(frame.message_id & 0xff);
  out[8] = static_cast<uint8_t>((frame.message_id >> 8) & 0xff);
  out[9] = static_cast<uint8_t>((frame.message_id >> 16) & 0xff);
  mavlink_memcpy_s(out + 10, out_capacity - 10, frame.payload, payload_len);

  crc_x25_t crc;
  mavlink_crc_x25_init(crc);
  mavlink_crc_x25_accumulate(crc, out[1]);
  mavlink_crc_x25_accumulate(crc, out[2]);
  mavlink_crc_x25_accumulate(crc, out[3]);
  mavlink_crc_x25_accumulate(crc, out[4]);
  mavlink_crc_x25_accumulate(crc, out[5]);
  mavlink_crc_x25_accumulate(crc, out[6]);
  mavlink_crc_x25_accumulate(crc, out[7]);
  mavlink_crc_x25_accumulate(crc, out[8]);
  mavlink_crc_x25_accumulate(crc, out[9]);
  for (size_t i = 0; i < payload_len; i++) {
    mavlink_crc_x25_accumulate(crc, out[10 + i]);
  }
  mavlink_crc_x25_accumulate(crc, frame.crc_extra);

  out[frame_len - 2] = static_cast<uint8_t>(mavlink_crc_x25_value(crc) & 0xff);
  out[frame_len - 1] = static_cast<uint8_t>((mavlink_crc_x25_value(crc) >> 8) & 0xff);
  return frame_len;
}

inline void mavlink_frame_init_v2(
  frame_t& frame,
  uint8_t sequence,
  uint8_t system_id,
  uint8_t component_id,
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t* payload,
  size_t payload_len
) {
  frame.version = MAVLINK_VERSION_V2;
  frame.sequence = sequence;
  frame.system_id = system_id;
  frame.component_id = component_id;
  frame.message_id = message_id;
  frame.payload_len = payload_len > 255 ? 255 : payload_len;
  mavlink_memset_s(frame.payload, sizeof(frame.payload), 0, sizeof(frame.payload));
  if (payload != nullptr && payload_len > 0) {
    mavlink_memcpy_s(
      frame.payload,
      sizeof(frame.payload),
      payload,
      frame.payload_len
    );
  }
  frame.crc_extra = crc_extra;
}

}  // namespace mavlink
