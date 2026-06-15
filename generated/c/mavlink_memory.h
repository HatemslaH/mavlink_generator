#ifndef MAVLINK_MEMORY_H
#define MAVLINK_MEMORY_H

#ifndef __STDC_WANT_LIB_EXT1__
#define __STDC_WANT_LIB_EXT1__ 1
#endif

#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#if defined(__STDC_LIB_EXT1__) || defined(_MSC_VER)

static inline void mavlink_memcpy_s(
  void *dest,
  size_t destsz,
  const void *src,
  size_t count
) {
  if (memcpy_s(dest, destsz, src, count) != 0) {
    if (dest != NULL && destsz > 0) {
      memset_s(dest, destsz, 0, destsz);
    }
  }
}

static inline void mavlink_memset_s(void *dest, size_t destsz, int ch, size_t count) {
  (void)memset_s(dest, destsz, ch, count);
}

static inline void mavlink_strncpy_s(
  char *dest,
  size_t destsz,
  const char *src,
  size_t count
) {
  (void)strncpy_s(dest, destsz, src, count);
}

#else

static inline void mavlink_memcpy_s(
  void *dest,
  size_t destsz,
  const void *src,
  size_t count
) {
  if (dest == NULL || destsz == 0) {
    return;
  }
  if (src == NULL && count > 0) {
    return;
  }
  if (count > destsz) {
    count = destsz;
  }
  const uint8_t *src_bytes = (const uint8_t *)src;
  uint8_t *dest_bytes = (uint8_t *)dest;
  for (size_t i = 0; i < count; i++) {
    dest_bytes[i] = src_bytes[i];
  }
}

static inline void mavlink_memset_s(void *dest, size_t destsz, int ch, size_t count) {
  if (dest == NULL || destsz == 0) {
    return;
  }
  if (count > destsz) {
    count = destsz;
  }
  uint8_t *dest_bytes = (uint8_t *)dest;
  uint8_t byte = (uint8_t)ch;
  for (size_t i = 0; i < count; i++) {
    dest_bytes[i] = byte;
  }
}

static inline void mavlink_strncpy_s(
  char *dest,
  size_t destsz,
  const char *src,
  size_t count
) {
  if (dest == NULL || destsz == 0) {
    return;
  }
  if (src == NULL) {
    dest[0] = '\0';
    return;
  }
  size_t max_copy = destsz - 1;
  if (count < max_copy) {
    max_copy = count;
  }
  for (size_t i = 0; i < max_copy && src[i] != '\0'; i++) {
    dest[i] = src[i];
  }
  dest[max_copy] = '\0';
}

#endif

#endif
