#ifndef MAVLINK_CRC_H
#define MAVLINK_CRC_H

#include "types.h"

typedef struct {
  uint16_t crc;
} mavlink_crc_x25_t;

static inline void mavlink_crc_x25_init(mavlink_crc_x25_t *crc) {
  crc->crc = 0xffff;
}

static inline uint16_t mavlink_crc_x25_value(const mavlink_crc_x25_t *crc) {
  return crc->crc & 0xffff;
}

static inline void mavlink_crc_x25_accumulate(mavlink_crc_x25_t *crc, uint8_t byte) {
  uint8_t tmp = byte ^ (crc->crc & 0xff);
  tmp &= 0xff;
  tmp ^= (uint8_t)((tmp << 4) & 0xff);
  crc->crc = (uint16_t)((crc->crc >> 8) ^ ((uint16_t)tmp << 8) ^ ((uint16_t)tmp << 3) ^ (tmp >> 4));
}

#endif
