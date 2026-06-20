#include <cstdio>

#include "protocols_common.hpp"

/// Command protocol example for the `rt_rc` dialect.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  auto link = mavlink::create_virtual_link(dialect);
  mavlink::CommandServer command_server(
    link.drone.get(),
    [](const mavlink::command_long_t& command) {
      std::printf(
        "Vehicle received COMMAND_LONG: %u p1=%f p2=%f\n",
        static_cast<unsigned>(command.command),
        command.param1,
        command.param2
      );
      return mavlink::MAV_RESULT_ACCEPTED;
    }
  );

  mavlink::CommandProtocol command_protocol(
    link.gcs.get(),
    mavlink::drone_system_id,
    mavlink::drone_component_id
  );

  const auto interval_ack = command_protocol.set_message_interval(
    mavlink::attitude_MSG_ID,
    100000
  );
  std::printf("SET_MESSAGE_INTERVAL ack: %d\n", static_cast<int>(interval_ack.result));

  const auto request_ack = command_protocol.request_message(mavlink::attitude_MSG_ID);
  std::printf("REQUEST_MESSAGE ack: %d\n", static_cast<int>(request_ack.result));

  const auto arm_ack = command_protocol.arm();
  std::printf("ARM ack: %d\n", static_cast<int>(arm_ack.result));

  const auto disarm_ack = command_protocol.disarm();
  std::printf("DISARM ack: %d\n", static_cast<int>(disarm_ack.result));

  command_server.close();
  mavlink::close_virtual_link(link);
  return 0;
}
