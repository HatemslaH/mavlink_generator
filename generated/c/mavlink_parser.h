#ifndef MAVLINK_PARSER_H
#define MAVLINK_PARSER_H

#include "crc.h"
#include "mavlink_dialect.h"
#include "mavlink_frame.h"
#include "mavlink_version.h"

typedef enum {
  MAVLINK_PARSER_INIT,
  MAVLINK_PARSER_WAIT_PAYLOAD_LENGTH,
  MAVLINK_PARSER_WAIT_INCOMPATIBILITY_FLAGS,
  MAVLINK_PARSER_WAIT_COMPATIBILITY_FLAGS,
  MAVLINK_PARSER_WAIT_PACKET_SEQUENCE,
  MAVLINK_PARSER_WAIT_SYSTEM_ID,
  MAVLINK_PARSER_WAIT_COMPONENT_ID,
  MAVLINK_PARSER_WAIT_MESSAGE_ID_LOW,
  MAVLINK_PARSER_WAIT_MESSAGE_ID_MIDDLE,
  MAVLINK_PARSER_WAIT_MESSAGE_ID_HIGH,
  MAVLINK_PARSER_WAIT_PAYLOAD_END,
  MAVLINK_PARSER_WAIT_CRC_LOW_BYTE,
  MAVLINK_PARSER_WAIT_CRC_HIGH_BYTE,
} mavlink_parser_state_t;

typedef struct {
  mavlink_parser_state_t state;
  mavlink_version_t version;
  uint8_t payload_length;
  uint8_t incompatibility_flags;
  uint8_t compatibility_flags;
  uint8_t sequence;
  uint8_t system_id;
  uint8_t component_id;
  uint8_t message_id_low;
  uint8_t message_id_middle;
  uint8_t message_id_high;
  uint32_t message_id;
  uint8_t payload[255];
  size_t payload_cursor;
  uint8_t crc_low_byte;
  uint8_t crc_high_byte;
  const mavlink_dialect_t *dialect;
} mavlink_parser_t;

static inline void mavlink_parser_init(mavlink_parser_t *parser, const mavlink_dialect_t *dialect) {
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
  parser->dialect = dialect;
}

static inline bool mavlink_parser_check_crc(const mavlink_parser_t *parser) {
  mavlink_crc_x25_t crc;
  mavlink_crc_x25_init(&crc);

  if (parser->version == MAVLINK_VERSION_V1) {
    mavlink_crc_x25_accumulate(&crc, parser->payload_length);
    mavlink_crc_x25_accumulate(&crc, parser->sequence);
    mavlink_crc_x25_accumulate(&crc, parser->system_id);
    mavlink_crc_x25_accumulate(&crc, parser->component_id);
    mavlink_crc_x25_accumulate(&crc, (uint8_t)parser->message_id);
  } else {
    mavlink_crc_x25_accumulate(&crc, parser->payload_length);
    mavlink_crc_x25_accumulate(&crc, parser->incompatibility_flags);
    mavlink_crc_x25_accumulate(&crc, parser->compatibility_flags);
    mavlink_crc_x25_accumulate(&crc, parser->sequence);
    mavlink_crc_x25_accumulate(&crc, parser->system_id);
    mavlink_crc_x25_accumulate(&crc, parser->component_id);
    mavlink_crc_x25_accumulate(&crc, parser->message_id_low);
    mavlink_crc_x25_accumulate(&crc, parser->message_id_middle);
    mavlink_crc_x25_accumulate(&crc, parser->message_id_high);
  }

  for (size_t i = 0; i < parser->payload_length; i++) {
    mavlink_crc_x25_accumulate(&crc, parser->payload[i]);
  }

  int crc_extra = parser->dialect->crc_extra(parser->dialect, parser->message_id);
  if (crc_extra < 0) {
    return false;
  }
  mavlink_crc_x25_accumulate(&crc, (uint8_t)crc_extra);

  uint16_t expected = (uint16_t)((parser->crc_high_byte << 8) | parser->crc_low_byte);
  return mavlink_crc_x25_value(&crc) == expected;
}

#endif
