#pragma once

#include "crc.hpp"
#include "mavlink_dialect.hpp"
#include "mavlink_frame.hpp"
#include "mavlink_version.hpp"

namespace mavlink {

enum class parser_state_t {
  INIT,
  WAIT_PAYLOAD_LENGTH,
  WAIT_INCOMPATIBILITY_FLAGS,
  WAIT_COMPATIBILITY_FLAGS,
  WAIT_PACKET_SEQUENCE,
  WAIT_SYSTEM_ID,
  WAIT_COMPONENT_ID,
  WAIT_MESSAGE_ID_LOW,
  WAIT_MESSAGE_ID_MIDDLE,
  WAIT_MESSAGE_ID_HIGH,
  WAIT_PAYLOAD_END,
  WAIT_CRC_LOW_BYTE,
  WAIT_CRC_HIGH_BYTE,
};

struct parser_t {
  parser_state_t state;
  version_t version;
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
  const dialect_t* dialect;
};

inline void mavlink_parser_init(parser_t& parser, const dialect_t* dialect) {
  parser.state = parser_state_t::INIT;
  parser.version = MAVLINK_VERSION_V1;
  parser.payload_length = 0;
  parser.incompatibility_flags = 0;
  parser.compatibility_flags = 0;
  parser.sequence = 0;
  parser.system_id = 0;
  parser.component_id = 0;
  parser.message_id_low = 0;
  parser.message_id_middle = 0;
  parser.message_id_high = 0;
  parser.message_id = 0;
  parser.payload_cursor = 0;
  parser.crc_low_byte = 0;
  parser.crc_high_byte = 0;
  parser.dialect = dialect;
}

inline bool mavlink_parser_check_crc(const parser_t& parser) {
  crc_x25_t crc;
  mavlink_crc_x25_init(crc);

  if (parser.version == MAVLINK_VERSION_V1) {
    mavlink_crc_x25_accumulate(crc, parser.payload_length);
    mavlink_crc_x25_accumulate(crc, parser.sequence);
    mavlink_crc_x25_accumulate(crc, parser.system_id);
    mavlink_crc_x25_accumulate(crc, parser.component_id);
    mavlink_crc_x25_accumulate(crc, static_cast<uint8_t>(parser.message_id));
  } else {
    mavlink_crc_x25_accumulate(crc, parser.payload_length);
    mavlink_crc_x25_accumulate(crc, parser.incompatibility_flags);
    mavlink_crc_x25_accumulate(crc, parser.compatibility_flags);
    mavlink_crc_x25_accumulate(crc, parser.sequence);
    mavlink_crc_x25_accumulate(crc, parser.system_id);
    mavlink_crc_x25_accumulate(crc, parser.component_id);
    mavlink_crc_x25_accumulate(crc, parser.message_id_low);
    mavlink_crc_x25_accumulate(crc, parser.message_id_middle);
    mavlink_crc_x25_accumulate(crc, parser.message_id_high);
  }

  for (size_t i = 0; i < parser.payload_length; i++) {
    mavlink_crc_x25_accumulate(crc, parser.payload[i]);
  }

  int crc_extra = parser.dialect->crc_extra(parser.dialect, parser.message_id);
  if (crc_extra < 0) {
    return false;
  }
  mavlink_crc_x25_accumulate(crc, static_cast<uint8_t>(crc_extra));

  uint16_t expected = static_cast<uint16_t>((parser.crc_high_byte << 8) | parser.crc_low_byte);
  return mavlink_crc_x25_value(crc) == expected;
}

}  // namespace mavlink
