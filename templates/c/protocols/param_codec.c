#include "param_codec.h"

#include <string.h>

#include "../mavlink_memory.h"

static float mavlink_param_codec_encode_int32_bits(int32_t value) {
  float encoded;
  mavlink_memcpy_s(&encoded, sizeof(encoded), &value, sizeof(value));
  return encoded;
}

static float mavlink_param_codec_encode_uint32_bits(uint32_t value) {
  float encoded;
  mavlink_memcpy_s(&encoded, sizeof(encoded), &value, sizeof(value));
  return encoded;
}

float mavlink_param_codec_encode_int8(int8_t value) {
  return mavlink_param_codec_encode_int32_bits((int32_t)value);
}

int8_t mavlink_param_codec_decode_int8(float encoded) {
  return (int8_t)mavlink_param_codec_decode_int32(encoded);
}

float mavlink_param_codec_encode_uint8(uint8_t value) {
  return mavlink_param_codec_encode_uint32_bits((uint32_t)value);
}

uint8_t mavlink_param_codec_decode_uint8(float encoded) {
  return (uint8_t)mavlink_param_codec_decode_uint32(encoded);
}

float mavlink_param_codec_encode_int16(int16_t value) {
  return mavlink_param_codec_encode_int32_bits((int32_t)value);
}

int16_t mavlink_param_codec_decode_int16(float encoded) {
  return (int16_t)mavlink_param_codec_decode_int32(encoded);
}

float mavlink_param_codec_encode_uint16(uint16_t value) {
  return mavlink_param_codec_encode_uint32_bits((uint32_t)value);
}

uint16_t mavlink_param_codec_decode_uint16(float encoded) {
  return (uint16_t)mavlink_param_codec_decode_uint32(encoded);
}

float mavlink_param_codec_encode_int32(int32_t value) {
  return mavlink_param_codec_encode_int32_bits(value);
}

int32_t mavlink_param_codec_decode_int32(float encoded) {
  int32_t value;
  mavlink_memcpy_s(&value, sizeof(value), &encoded, sizeof(encoded));
  return value;
}

float mavlink_param_codec_encode_uint32(uint32_t value) {
  return mavlink_param_codec_encode_uint32_bits(value);
}

uint32_t mavlink_param_codec_decode_uint32(float encoded) {
  uint32_t value;
  mavlink_memcpy_s(&value, sizeof(value), &encoded, sizeof(encoded));
  return value;
}

float mavlink_param_codec_encode_float(float value) {
  return value;
}

float mavlink_param_codec_decode_float(float encoded) {
  return encoded;
}

float mavlink_param_codec_encode_value(double value, mavlink_param_type_t type) {
  switch (type) {
  case MAVLINK_PARAM_TYPE_UINT8:
    return mavlink_param_codec_encode_uint8((uint8_t)value);
  case MAVLINK_PARAM_TYPE_INT8:
    return mavlink_param_codec_encode_int8((int8_t)value);
  case MAVLINK_PARAM_TYPE_UINT16:
    return mavlink_param_codec_encode_uint16((uint16_t)value);
  case MAVLINK_PARAM_TYPE_INT16:
    return mavlink_param_codec_encode_int16((int16_t)value);
  case MAVLINK_PARAM_TYPE_INT32:
    return mavlink_param_codec_encode_int32((int32_t)value);
  case MAVLINK_PARAM_TYPE_UINT32:
    return mavlink_param_codec_encode_uint32((uint32_t)value);
  case MAVLINK_PARAM_TYPE_REAL32:
  default:
    return mavlink_param_codec_encode_float((float)value);
  }
}

double mavlink_param_codec_decode_value(float encoded, mavlink_param_type_t type) {
  switch (type) {
  case MAVLINK_PARAM_TYPE_UINT8:
    return (double)mavlink_param_codec_decode_uint8(encoded);
  case MAVLINK_PARAM_TYPE_INT8:
    return (double)mavlink_param_codec_decode_int8(encoded);
  case MAVLINK_PARAM_TYPE_UINT16:
    return (double)mavlink_param_codec_decode_uint16(encoded);
  case MAVLINK_PARAM_TYPE_INT16:
    return (double)mavlink_param_codec_decode_int16(encoded);
  case MAVLINK_PARAM_TYPE_INT32:
    return (double)mavlink_param_codec_decode_int32(encoded);
  case MAVLINK_PARAM_TYPE_UINT32:
    return (double)mavlink_param_codec_decode_uint32(encoded);
  case MAVLINK_PARAM_TYPE_REAL32:
  default:
    return (double)mavlink_param_codec_decode_float(encoded);
  }
}

void mavlink_param_codec_param_id_from_string(char out[16], const char *name) {
  mavlink_memset_s(out, 16, 0, 16);
  if (name != NULL) {
    mavlink_strncpy_s(out, 16, name, 15);
  }
}

void mavlink_param_codec_param_id_to_string(const char id[16], char *out, size_t out_len) {
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
