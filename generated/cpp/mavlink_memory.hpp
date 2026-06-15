#pragma once

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace mavlink {

inline void mavlink_memcpy_s(
  void* dest,
  size_t destsz,
  const void* src,
  size_t count
) {
  if (dest == nullptr || destsz == 0) {
    return;
  }
  if (src == nullptr && count > 0) {
    return;
  }
  if (count > destsz) {
    count = destsz;
  }
  std::memcpy(dest, src, count);
}

inline void mavlink_memset_s(void* dest, size_t destsz, int ch, size_t count) {
  if (dest == nullptr || destsz == 0) {
    return;
  }
  if (count > destsz) {
    count = destsz;
  }
  std::memset(dest, ch, count);
}

inline void mavlink_strncpy_s(
  char* dest,
  size_t destsz,
  const char* src,
  size_t count
) {
  if (dest == nullptr || destsz == 0) {
    return;
  }
  if (src == nullptr) {
    dest[0] = '\0';
    return;
  }
  size_t max_copy = destsz - 1;
  if (count < max_copy) {
    max_copy = count;
  }
  std::strncpy(dest, src, max_copy);
  dest[max_copy] = '\0';
}

}  // namespace mavlink
