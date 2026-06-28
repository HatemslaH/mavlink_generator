#pragma once

#include <memory>

#include "../mavlink_protocols.hpp"

namespace mavlink {

/// Ground control station identity (MAVLink convention).
inline constexpr uint8_t gcs_system_id = 255;
inline constexpr uint8_t gcs_component_id = 190;

/// Simulated autopilot identity.
inline constexpr uint8_t drone_system_id = 1;
inline constexpr uint8_t drone_component_id = 1;

struct VirtualLink {
  std::shared_ptr<VirtualMavlinkBus> bus;
  std::unique_ptr<MavlinkSession> gcs;
  std::unique_ptr<MavlinkSession> drone;
  const dialect_t* dialect;
};

/// Create a linked GCS/drone pair over an in-memory MAVLink bus.
template<typename DialectT>
VirtualLink create_virtual_link(DialectT& dialect) {
  auto bus = std::make_shared<VirtualMavlinkBus>();
  auto gcs_link = bus->create_endpoint();
  auto drone_link = bus->create_endpoint();

  VirtualLink link{};
  link.bus = bus;
  link.dialect = &dialect.base;
  link.gcs = std::make_unique<MavlinkSession>(
    link.dialect,
    gcs_link,
    gcs_system_id,
    gcs_component_id
  );
  link.drone = std::make_unique<MavlinkSession>(
    link.dialect,
    drone_link,
    drone_system_id,
    drone_component_id
  );
  return link;
}

inline void close_virtual_link(VirtualLink& link) {
  if (link.gcs) {
    link.gcs->close();
    link.gcs.reset();
  }
  if (link.drone) {
    link.drone->close();
    link.drone.reset();
  }
  if (link.bus) {
    link.bus->close_all();
    link.bus.reset();
  }
}

}  // namespace mavlink
