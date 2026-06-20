#include <chrono>
#include <cstdio>
#include <thread>
#include <vector>

#include "protocols_common.hpp"

/// Typed message subscription example for the `rt_rc` dialect.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  auto link = mavlink::create_virtual_link(dialect);
  std::vector<mavlink::attitude_t> attitude_samples;

  auto subscription = link.gcs->listen_message(
    mavlink::attitude_MSG_ID,
    [&](const uint8_t* payload, size_t, const mavlink::frame_t&) {
      mavlink::attitude_t attitude{};
      mavlink::attitude_parse(payload, attitude);
      attitude_samples.push_back(attitude);
    },
    mavlink::drone_system_id
  );

  mavlink::attitude_t attitude{};
  attitude.time_boot_ms = 1000;
  attitude.roll = 0.1f;
  attitude.pitch = -0.05f;
  attitude.yaw = 1.57f;
  uint8_t payload[mavlink::attitude_ENCODED_LENGTH];
  mavlink::attitude_serialize(attitude, payload);
  link.drone->send_frame(
    mavlink::attitude_MSG_ID,
    mavlink::attitude_CRC_EXTRA,
    payload,
    mavlink::attitude_ENCODED_LENGTH
  );

  std::this_thread::sleep_for(std::chrono::milliseconds(50));
  subscription.cancel();

  std::printf(
    "Received %zu ATTITUDE samples via listen_message\n",
    attitude_samples.size()
  );
  if (!attitude_samples.empty()) {
    const auto& sample = attitude_samples.front();
    std::printf(
      "  roll=%f pitch=%f yaw=%f\n",
      sample.roll,
      sample.pitch,
      sample.yaw
    );
  }

  mavlink::close_virtual_link(link);
  return 0;
}
