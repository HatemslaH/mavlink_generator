#pragma once

#include <memory>

#include "mavlink_protocols.hpp"

namespace sitl_gcs {

/// Ground control station identity (MAVLink convention).
inline constexpr uint8_t kGcsSystemId = 255;
inline constexpr uint8_t kGcsComponentId = 190;

/// Shared MAVLink GCS state for the interactive SITL example.
struct GcsContext {
  mavlink::MavlinkGcs& gcs;
  mavlink::MavlinkNode vehicle;
  mavlink::MavlinkVehicleClient client;

  /// Cancels in-flight parameter/mission operations (type `cancel` in CLI).
  std::unique_ptr<mavlink::MavlinkCancellationToken> operation_cancel;

  mavlink::MavlinkSession* session() const { return gcs.session(); }
  mavlink::HeartbeatMonitor* heartbeat_monitor() const { return gcs.heartbeat_monitor(); }
  mavlink::ParameterProtocol& parameters() { return client.parameters(); }
  mavlink::MissionProtocol& mission() { return client.mission(); }
  mavlink::CommandProtocol& command() { return client.command(); }
  uint8_t target_system() const { return vehicle.system_id; }
  uint8_t target_component() const { return vehicle.component_id; }
};

}  // namespace sitl_gcs
