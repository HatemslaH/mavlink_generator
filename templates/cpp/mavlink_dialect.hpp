#pragma once

#include "types.hpp"

namespace mavlink {

struct dialect_t {
  int version;
  bool (*parse)(
    const dialect_t* dialect,
    uint32_t message_id,
    const uint8_t* payload,
    size_t payload_len,
    void* out_message
  );
  int (*crc_extra)(const dialect_t* dialect, uint32_t message_id);
};

}  // namespace mavlink
