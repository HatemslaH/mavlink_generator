#include <cstdio>
#include <vector>

#include "protocols_common.hpp"
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  auto link = mavlink::create_virtual_link(dialect);
  mavlink::MissionServer mission_server(link.drone.get());
  mavlink::CommandServer command_server(link.drone.get());
  mavlink::MissionProtocol mission_protocol(
    link.gcs.get(),
    mavlink::drone_system_id,
    mavlink::drone_component_id
  );

  const std::vector<mavlink::mission_item_int_t> plan = {
    mavlink::MissionItems::waypoint(
      0, 47.397742, 8.545594, 50,
      mavlink::drone_system_id, mavlink::drone_component_id
    ),
    mavlink::MissionItems::waypoint(
      1, 47.398000, 8.546000, 50,
      mavlink::drone_system_id, mavlink::drone_component_id
    ),
  };

  const auto upload_result = mission_protocol.upload(
    plan,
    mavlink::MAV_MISSION_TYPE_MISSION,
    [](int sent, int total, const mavlink::mission_item_int_t& item) {
      std::printf(
        "Upload progress %d/%d seq=%u cmd=%u\n",
        sent,
        total,
        item.seq,
        static_cast<unsigned>(item.command)
      );
    }
  );
  std::printf("Mission upload result: %d\n", static_cast<int>(upload_result));
  std::printf("Vehicle stored %zu items\n", mission_server.items().size());

  const auto downloaded = mission_protocol.download(
    mavlink::MAV_MISSION_TYPE_MISSION,
    [](int received, int total, const mavlink::mission_item_int_t& item) {
      std::printf("Download progress %d/%d seq=%u\n", received, total, item.seq);
    }
  );
  std::printf("Downloaded %zu mission items\n", downloaded.size());

  mavlink::CommandProtocol command_protocol(
    link.gcs.get(),
    mavlink::drone_system_id,
    mavlink::drone_component_id
  );
  const auto set_current = mission_protocol.set_current_with_command(0, &command_protocol);
  std::printf(
    "Set current seq=%u ack=%d\n",
    set_current.sequence,
    set_current.command_ack.has_value()
      ? static_cast<int>(set_current.command_ack->result)
      : -1
  );

  const auto clear_result = mission_protocol.clear();
  std::printf("Mission clear result: %d\n", static_cast<int>(clear_result));

  mission_server.close();
  command_server.close();
  mavlink::close_virtual_link(link);
  return 0;
}
