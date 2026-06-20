#include <chrono>
#include <cstdio>
#include <map>
#include <memory>
#include <set>

#include "protocols_common.hpp"

/// MavlinkGcs / MavlinkVehicleClient facade example for `rt_rc`.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  auto bus = std::make_shared<mavlink::VirtualMavlinkBus>();
  auto gcs_link = bus->create_endpoint();
  auto drone_link = bus->create_endpoint();

  mavlink::MavlinkGcs gcs = mavlink::MavlinkGcs::connect(
    &dialect.base,
    gcs_link,
    mavlink::gcs_system_id,
    mavlink::gcs_component_id
  );

  auto drone_session = std::make_unique<mavlink::MavlinkSession>(
    &dialect.base,
    drone_link,
    mavlink::drone_system_id,
    mavlink::drone_component_id
  );

  mavlink::HeartbeatPublisher drone_publisher(
    drone_session.get(),
    mavlink::HeartbeatTemplates::autopilot(dialect.base.version),
    std::chrono::milliseconds(500)
  );

  const std::map<std::string, mavlink::ParamStoredValue> initial_values = {
    {"SYSID_THISMAV", {1.0, mavlink::MAV_PARAM_TYPE_INT32}},
  };
  mavlink::ParameterServer parameter_server(drone_session.get(), &initial_values);
  mavlink::CommandServer command_server(drone_session.get());

  gcs.start();
  drone_publisher.start();

  const std::set<uint8_t> exclude = {mavlink::gcs_system_id};
  mavlink::MavlinkVehicleClient client = gcs.wait_for_vehicle(&exclude);
  std::printf(
    "Connected to vehicle %u:%u\n",
    client.vehicle().system_id,
    client.vehicle().component_id
  );

  const auto params = client.parameters().fetch_all();
  std::printf("Vehicle has %zu parameters\n", params.size());

  const auto ack = client.command().request_message(mavlink::heartbeat_MSG_ID);
  std::printf("REQUEST_MESSAGE ack: %d\n", static_cast<int>(ack.result));

  parameter_server.close();
  command_server.close();
  drone_publisher.stop();
  drone_session->close();
  gcs.close();
  bus->close_all();
  return 0;
}
