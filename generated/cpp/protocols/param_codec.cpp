#include "param_codec.hpp"

#include "../mavlink_memory.hpp"

#include <cstring>

namespace mavlink {

float ParamCodec::encode_int8(int8_t value) { return encode_int32(value); }

int8_t ParamCodec::decode_int8(float encoded) {
  return static_cast<int8_t>(decode_int32(encoded));
}

float ParamCodec::encode_uint8(uint8_t value) { return encode_uint32(value); }

uint8_t ParamCodec::decode_uint8(float encoded) {
  return static_cast<uint8_t>(decode_uint32(encoded));
}

float ParamCodec::encode_int16(int16_t value) { return encode_int32(value); }

int16_t ParamCodec::decode_int16(float encoded) {
  return static_cast<int16_t>(decode_int32(encoded));
}

float ParamCodec::encode_uint16(uint16_t value) { return encode_uint32(value); }

uint16_t ParamCodec::decode_uint16(float encoded) {
  return static_cast<uint16_t>(decode_uint32(encoded));
}

float ParamCodec::encode_int32(int32_t value) {
  float encoded;
  mavlink_memcpy_s(&encoded, sizeof(encoded), &value, sizeof(value));
  return encoded;
}

int32_t ParamCodec::decode_int32(float encoded) {
  int32_t value;
  mavlink_memcpy_s(&value, sizeof(value), &encoded, sizeof(encoded));
  return value;
}

float ParamCodec::encode_uint32(uint32_t value) {
  float encoded;
  mavlink_memcpy_s(&encoded, sizeof(encoded), &value, sizeof(value));
  return encoded;
}

uint32_t ParamCodec::decode_uint32(float encoded) {
  uint32_t value;
  mavlink_memcpy_s(&value, sizeof(value), &encoded, sizeof(encoded));
  return value;
}

float ParamCodec::encode_float(float value) { return value; }

float ParamCodec::decode_float(float encoded) { return encoded; }

float ParamCodec::encode_value(double value, MAV_PARAM_TYPE type) {
  switch (type) {
    case MAV_PARAM_TYPE_UINT8:
      return encode_uint8(static_cast<uint8_t>(value));
    case MAV_PARAM_TYPE_INT8:
      return encode_int8(static_cast<int8_t>(value));
    case MAV_PARAM_TYPE_UINT16:
      return encode_uint16(static_cast<uint16_t>(value));
    case MAV_PARAM_TYPE_INT16:
      return encode_int16(static_cast<int16_t>(value));
    case MAV_PARAM_TYPE_INT32:
      return encode_int32(static_cast<int32_t>(value));
    case MAV_PARAM_TYPE_UINT32:
      return encode_uint32(static_cast<uint32_t>(value));
    case MAV_PARAM_TYPE_REAL32:
      return encode_float(static_cast<float>(value));
    default:
      return static_cast<float>(value);
  }
}

double ParamCodec::decode_value(float encoded, MAV_PARAM_TYPE type) {
  switch (type) {
    case MAV_PARAM_TYPE_UINT8:
      return decode_uint8(encoded);
    case MAV_PARAM_TYPE_INT8:
      return decode_int8(encoded);
    case MAV_PARAM_TYPE_UINT16:
      return decode_uint16(encoded);
    case MAV_PARAM_TYPE_INT16:
      return decode_int16(encoded);
    case MAV_PARAM_TYPE_INT32:
      return decode_int32(encoded);
    case MAV_PARAM_TYPE_UINT32:
      return decode_uint32(encoded);
    case MAV_PARAM_TYPE_REAL32:
      return decode_float(encoded);
    default:
      return encoded;
  }
}

void ParamCodec::param_id_from_string(char out[16], const char* name) {
  mavlink_memset_s(out, 16, 0, 16);
  if (name != nullptr) {
    mavlink_strncpy_s(out, 16, name, 15);
  }
}

void ParamCodec::param_id_to_string(const char id[16], char* out, size_t out_len) {
  if (out == nullptr || out_len == 0) {
    return;
  }
  size_t end = 0;
  while (end < 16 && id[end] != '\0') {
    end++;
  }
  size_t copy_len = end < out_len - 1 ? end : out_len - 1;
  mavlink_memcpy_s(out, out_len, id, copy_len);
  out[copy_len] = '\0';
}

}  // namespace mavlink
