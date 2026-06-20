#include <chrono>
#include <cstdio>
#include <thread>

#include "protocols_common.hpp"

/// Heartbeat protocol example for the `rt_rc` dialect.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  auto link = mavlink::create_virtual_link(dialect);

  mavlink::HeartbeatPublisher gcs_publisher(
    link.gcs.get(),
    mavlink::HeartbeatTemplates::gcs(dialect.base.version),
    std::chrono::milliseconds(500)
  );
  mavlink::HeartbeatPublisher drone_publisher(
    link.drone.get(),
    mavlink::HeartbeatTemplates::autopilot(dialect.base.version),
    std::chrono::milliseconds(500)
  );
  mavlink::HeartbeatMonitor gcs_monitor(link.gcs.get(), std::chrono::seconds(2));

  gcs_monitor.start();
  gcs_publisher.start();
  drone_publisher.start();

  const mavlink::MavlinkNode vehicle = gcs_monitor.wait_for_vehicle(
    nullptr,
    std::chrono::seconds(5)
  );
  std::printf(
    "Vehicle discovered: %u:%u\n",
    vehicle.system_id,
    vehicle.component_id
  );
  std::printf("Drone online: %s\n", gcs_monitor.is_online(vehicle) ? "true" : "false");

  const auto state = gcs_monitor.state_for(vehicle);
  if (state.has_value()) {
    std::printf(
      "Drone heartbeat: type=%d status=%d\n",
      static_cast<int>(state->heartbeat_msg.type),
      static_cast<int>(state->heartbeat_msg.system_status)
    );
  }

  drone_publisher.stop();
  std::this_thread::sleep_for(std::chrono::milliseconds(2500));
  std::printf(
    "Drone online after stop: %s\n",
    gcs_monitor.is_online(vehicle) ? "true" : "false"
  );

  gcs_monitor.stop();
  gcs_publisher.stop();
  mavlink::close_virtual_link(link);
  return 0;
}
