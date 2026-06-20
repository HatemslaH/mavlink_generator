#ifndef MAVLINK_SITL_SAMPLE_MISSION_H
#define MAVLINK_SITL_SAMPLE_MISSION_H

#include <stddef.h>

#include "mavlink.h"

/// Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).
size_t build_sample_mission(
  mission_item_int_t *out_items,
  size_t max_items,
  uint8_t target_system,
  uint8_t target_component
);

void describe_mission_item(const mission_item_int_t *item, char *buf, size_t buf_len);

#endif
