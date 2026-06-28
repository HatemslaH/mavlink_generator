#pragma once

#include <chrono>
#include <cstdint>
#include <memory>
#include <set>

#include "../mavlink_dialect.hpp"
#include "command_protocol.hpp"
#include "heartbeat_protocol.hpp"
#include "mavlink_link.hpp"
#include "mavlink_session.hpp"
#include "mission_protocol.hpp"
#include "parameter_protocol.hpp"

namespace mavlink {

/// Protocol clients bound to a single remote MAVLink vehicle.
class MavlinkVehicleClient {
 public:
  MavlinkVehicleClient(
    MavlinkSession* session,
    const MavlinkNode& vehicle,
    std::chrono::milliseconds parameter_request_timeout = std::chrono::seconds(10),
    std::chrono::milliseconds parameter_idle_timeout = std::chrono::seconds(2),
    std::chrono::milliseconds mission_item_timeout = std::chrono::seconds(10),
    std::chrono::milliseconds mission_operation_timeout = std::chrono::seconds(30),
    std::chrono::milliseconds command_timeout = std::chrono::seconds(10)
  );

  MavlinkSession* session() const { return session_; }
  const MavlinkNode& vehicle() const { return vehicle_; }
  ParameterProtocol& parameters() { return parameters_; }
  MissionProtocol& mission() { return mission_; }
  CommandProtocol& command() { return command_; }
  uint8_t target_system() const { return vehicle_.system_id; }
  uint8_t target_component() const { return vehicle_.component_id; }

 private:
  MavlinkSession* session_;
  MavlinkNode vehicle_;
  ParameterProtocol parameters_;
  MissionProtocol mission_;
  CommandProtocol command_;
};

/// Ground control station bootstrap: session, heartbeat publisher, and monitor.
class MavlinkGcs {
 public:
  MavlinkGcs(
    std::unique_ptr<MavlinkSession> session,
    std::unique_ptr<HeartbeatPublisher> heartbeat_publisher,
    std::unique_ptr<HeartbeatMonitor> heartbeat_monitor
  );

  static MavlinkGcs connect(
    const dialect_t* dialect,
    std::shared_ptr<MavlinkLink> link,
    uint8_t system_id = 255,
    uint8_t component_id = 190,
    std::chrono::milliseconds heartbeat_interval = std::chrono::seconds(1),
    std::chrono::milliseconds heartbeat_timeout = std::chrono::seconds(3)
  );

  MavlinkSession* session() const { return session_.get(); }
  HeartbeatPublisher* heartbeat_publisher() const { return heartbeat_publisher_.get(); }
  HeartbeatMonitor* heartbeat_monitor() const { return heartbeat_monitor_.get(); }

  void start();
  void stop_heartbeats();
  MavlinkVehicleClient wait_for_vehicle(
    const std::set<uint8_t>* exclude_system_ids = nullptr,
    std::chrono::milliseconds timeout = std::chrono::seconds(60)
  );
  MavlinkVehicleClient vehicle_client(const MavlinkNode& vehicle);
  void close();

 private:
  std::unique_ptr<MavlinkSession> session_;
  std::unique_ptr<HeartbeatPublisher> heartbeat_publisher_;
  std::unique_ptr<HeartbeatMonitor> heartbeat_monitor_;
};

}  // namespace mavlink
