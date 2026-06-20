#include "mavlink_vehicle_client.hpp"

namespace mavlink {

MavlinkVehicleClient::MavlinkVehicleClient(
  MavlinkSession* session,
  const MavlinkNode& vehicle,
  std::chrono::milliseconds parameter_request_timeout,
  std::chrono::milliseconds parameter_idle_timeout,
  std::chrono::milliseconds mission_item_timeout,
  std::chrono::milliseconds mission_operation_timeout,
  std::chrono::milliseconds command_timeout
)
    : session_(session),
      vehicle_(vehicle),
      parameters_(session, vehicle.system_id, vehicle.component_id, parameter_idle_timeout, parameter_request_timeout),
      mission_(session, vehicle.system_id, vehicle.component_id, mission_item_timeout, mission_operation_timeout),
      command_(session, vehicle.system_id, vehicle.component_id, command_timeout) {}

MavlinkGcs::MavlinkGcs(
  std::unique_ptr<MavlinkSession> session,
  std::unique_ptr<HeartbeatPublisher> heartbeat_publisher,
  std::unique_ptr<HeartbeatMonitor> heartbeat_monitor
)
    : session_(std::move(session)),
      heartbeat_publisher_(std::move(heartbeat_publisher)),
      heartbeat_monitor_(std::move(heartbeat_monitor)) {}

MavlinkGcs MavlinkGcs::connect(
  const dialect_t* dialect,
  std::shared_ptr<MavlinkLink> link,
  uint8_t system_id,
  uint8_t component_id,
  std::chrono::milliseconds heartbeat_interval,
  std::chrono::milliseconds heartbeat_timeout
) {
  auto session = std::make_unique<MavlinkSession>(
    dialect,
    std::move(link),
    system_id,
    component_id
  );

  auto publisher = std::make_unique<HeartbeatPublisher>(
    session.get(),
    HeartbeatTemplates::gcs(dialect->version),
    heartbeat_interval
  );

  auto monitor = std::make_unique<HeartbeatMonitor>(session.get(), heartbeat_timeout);

  return MavlinkGcs(std::move(session), std::move(publisher), std::move(monitor));
}

void MavlinkGcs::start() {
  heartbeat_monitor_->start();
  heartbeat_publisher_->start();
}

void MavlinkGcs::stop_heartbeats() {
  heartbeat_publisher_->stop();
  heartbeat_monitor_->stop();
}

MavlinkVehicleClient MavlinkGcs::wait_for_vehicle(
  const std::set<uint8_t>* exclude_system_ids,
  std::chrono::milliseconds timeout
) {
  const MavlinkNode node = heartbeat_monitor_->wait_for_vehicle(exclude_system_ids, timeout);
  return vehicle_client(node);
}

MavlinkVehicleClient MavlinkGcs::vehicle_client(const MavlinkNode& vehicle) {
  return MavlinkVehicleClient(session_.get(), vehicle);
}

void MavlinkGcs::close() {
  stop_heartbeats();
  session_->close();
}

}  // namespace mavlink
