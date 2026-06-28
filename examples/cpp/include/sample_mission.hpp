#pragma once

#include <cstdio>
#include <string>
#include <vector>

#include "mavlink_protocols.hpp"

namespace sitl_gcs {

/// Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).
inline std::vector<mavlink::mission_item_int_t> build_sample_mission(
  uint8_t target_system,
  uint8_t target_component
) {
  return mavlink::MissionItems::with_sequential_seq({
    mavlink::MissionItems::waypoint(
      0, 47.397742, 8.545594, 50, target_system, target_component
    ),
    mavlink::MissionItems::waypoint(
      1, 47.398000, 8.546000, 50, target_system, target_component
    ),
    mavlink::MissionItems::waypoint(
      2, 47.398258, 8.546406, 50, target_system, target_component,
      mavlink::MAV_CMD_NAV_RETURN_TO_LAUNCH
    ),
  });
}

inline std::string describe_mission_item(const mavlink::mission_item_int_t& item) {
  const double lat = static_cast<double>(item.x) / 1e7;
  const double lon = static_cast<double>(item.y) / 1e7;
  char buffer[192];
  std::snprintf(
    buffer,
    sizeof(buffer),
    "seq=%u cmd=%d lat=%.6f lon=%.6f alt=%.0fm",
    item.seq,
    static_cast<int>(item.command),
    lat,
    lon,
    static_cast<double>(item.z)
  );
  return buffer;
}

}  // namespace sitl_gcs
