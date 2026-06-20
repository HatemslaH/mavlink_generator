#ifndef MAVLINK_PROTOCOLS_PARAM_CODEC_H
#define MAVLINK_PROTOCOLS_PARAM_CODEC_H

#include <stddef.h>
#include <stdint.h>

/// MAVLink parameter type (matches generated MavParamType enum values).
typedef uint8_t mavlink_param_type_t;

#define MAVLINK_PARAM_TYPE_UINT8 1
#define MAVLINK_PARAM_TYPE_INT8 2
#define MAVLINK_PARAM_TYPE_UINT16 3
#define MAVLINK_PARAM_TYPE_INT16 4
#define MAVLINK_PARAM_TYPE_UINT32 5
#define MAVLINK_PARAM_TYPE_INT32 6
#define MAVLINK_PARAM_TYPE_REAL32 9

float mavlink_param_codec_encode_int8(int8_t value);
int8_t mavlink_param_codec_decode_int8(float encoded);

float mavlink_param_codec_encode_uint8(uint8_t value);
uint8_t mavlink_param_codec_decode_uint8(float encoded);

float mavlink_param_codec_encode_int16(int16_t value);
int16_t mavlink_param_codec_decode_int16(float encoded);

float mavlink_param_codec_encode_uint16(uint16_t value);
uint16_t mavlink_param_codec_decode_uint16(float encoded);

float mavlink_param_codec_encode_int32(int32_t value);
int32_t mavlink_param_codec_decode_int32(float encoded);

float mavlink_param_codec_encode_uint32(uint32_t value);
uint32_t mavlink_param_codec_decode_uint32(float encoded);

float mavlink_param_codec_encode_float(float value);
float mavlink_param_codec_decode_float(float encoded);

float mavlink_param_codec_encode_value(double value, mavlink_param_type_t type);
double mavlink_param_codec_decode_value(float encoded, mavlink_param_type_t type);

void mavlink_param_codec_param_id_from_string(char out[16], const char *name);
void mavlink_param_codec_param_id_to_string(const char id[16], char *out, size_t out_len);

#endif
