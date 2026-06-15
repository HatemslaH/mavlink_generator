#pragma once

#include "types.hpp"

namespace mavlink {

struct crc_x25_t {
  uint16_t crc;
};

inline void mavlink_crc_x25_init(crc_x25_t& crc) {
  crc.crc = 0xffff;
}

inline uint16_t mavlink_crc_x25_value(const crc_x25_t& crc) {
  return crc.crc & 0xffff;
}

inline void mavlink_crc_x25_accumulate(crc_x25_t& crc, uint8_t byte) {
  uint8_t tmp = byte ^ static_cast<uint8_t>(crc.crc & 0xff);
  tmp &= 0xff;
  tmp ^= static_cast<uint8_t>((tmp << 4) & 0xff);
  crc.crc = static_cast<uint16_t>(
    (crc.crc >> 8) ^ (static_cast<uint16_t>(tmp) << 8) ^ (static_cast<uint16_t>(tmp) << 3) ^ (tmp >> 4)
  );
}

}  // namespace mavlink
