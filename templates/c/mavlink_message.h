#ifndef MAVLINK_MESSAGE_H
#define MAVLINK_MESSAGE_H

#include "mavlink_memory.h"
#include "types.h"

static inline int8_t mavlink_get_int8(const uint8_t *data, size_t offset) {
  return (int8_t)data[offset];
}

static inline uint8_t mavlink_get_uint8(const uint8_t *data, size_t offset) {
  return data[offset];
}

static inline int16_t mavlink_get_int16(const uint8_t *data, size_t offset) {
  int16_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline uint16_t mavlink_get_uint16(const uint8_t *data, size_t offset) {
  uint16_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline int32_t mavlink_get_int32(const uint8_t *data, size_t offset) {
  int32_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline uint32_t mavlink_get_uint32(const uint8_t *data, size_t offset) {
  uint32_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline int64_t mavlink_get_int64(const uint8_t *data, size_t offset) {
  int64_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline uint64_t mavlink_get_uint64(const uint8_t *data, size_t offset) {
  uint64_t value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline float mavlink_get_float(const uint8_t *data, size_t offset) {
  float value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline double mavlink_get_double(const uint8_t *data, size_t offset) {
  double value;
  mavlink_memcpy_s(&value, sizeof(value), data + offset, sizeof(value));
  return value;
}

static inline void mavlink_put_int8(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  int8_t value
) {
  if (offset < capacity) {
    data[offset] = (uint8_t)value;
  }
}

static inline void mavlink_put_uint8(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  uint8_t value
) {
  if (offset < capacity) {
    data[offset] = value;
  }
}

static inline void mavlink_put_int16(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  int16_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_uint16(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  uint16_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_int32(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  int32_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_uint32(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  uint32_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_int64(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  int64_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_uint64(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  uint64_t value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_float(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  float value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_put_double(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  double value
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    &value,
    sizeof(value)
  );
}

static inline void mavlink_get_bytes(
  const uint8_t *data,
  size_t offset,
  uint8_t *out,
  size_t out_capacity,
  size_t length
) {
  mavlink_memcpy_s(out, out_capacity, data + offset, length);
}

static inline void mavlink_put_bytes(
  uint8_t *data,
  size_t capacity,
  size_t offset,
  const uint8_t *value,
  size_t length
) {
  mavlink_memcpy_s(
    data + offset,
    capacity > offset ? capacity - offset : 0,
    value,
    length
  );
}

#endif
