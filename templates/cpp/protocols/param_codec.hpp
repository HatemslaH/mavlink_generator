#pragma once

#include <cstddef>
#include <cstdint>

#include "../mavlink.hpp"

namespace mavlink {

/// Byte-wise parameter value encoding per MAVLink parameter protocol.
///
/// See https://mavlink.io/en/services/parameter.html
class ParamCodec {
 public:
  ParamCodec() = delete;

  static float encode_int8(int8_t value);
  static int8_t decode_int8(float encoded);

  static float encode_uint8(uint8_t value);
  static uint8_t decode_uint8(float encoded);

  static float encode_int16(int16_t value);
  static int16_t decode_int16(float encoded);

  static float encode_uint16(uint16_t value);
  static uint16_t decode_uint16(float encoded);

  static float encode_int32(int32_t value);
  static int32_t decode_int32(float encoded);

  static float encode_uint32(uint32_t value);
  static uint32_t decode_uint32(float encoded);

  static float encode_float(float value);
  static float decode_float(float encoded);

  static float encode_value(double value, MAV_PARAM_TYPE type);
  static double decode_value(float encoded, MAV_PARAM_TYPE type);

  /// Encode a short ASCII parameter name into a MAVLink `char[16]` field.
  static void param_id_from_string(char out[16], const char* name);

  /// Decode a MAVLink `char[16]` parameter id to a string.
  static void param_id_to_string(const char id[16], char* out, size_t out_len);
};

}  // namespace mavlink
