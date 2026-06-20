#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <exception>
#include <iostream>
#include <memory>
#include <set>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#include "gcs_context.hpp"
#include "mavlink_protocols.hpp"
#include "port_picker.hpp"
#include "sample_mission.hpp"
#include "serial_link.hpp"

namespace {

using namespace sitl_gcs;

void print_help() {
  std::puts("Commands:");
  std::puts("  help              Show this help");
  std::puts("  hb                Heartbeat / link status");
  std::puts("  cancel            Cancel in-flight params/mission operation");
  std::puts("  params            Request full parameter list (with progress)");
  std::puts("  pr <name>         Read one parameter by name");
  std::puts("  pw <name> <value> Write parameter (type from cache or REAL32)");
  std::puts("  mu                Upload hardcoded sample mission");
  std::puts("  md                Download mission from vehicle");
  std::puts("  mc                Clear onboard mission");
  std::puts("  ms <seq>          Set active mission item (mission + command)");
  std::puts("  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)");
  std::puts("  si <msgId> <us>   Set message interval (microseconds)");
  std::puts("  att [seconds]     Stream ATTITUDE via listen_message (default 5 s)");
  std::puts("  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)");
  std::puts("  disarm [force]    Disarm motors");
  std::puts("  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH");
  std::puts("  quit              Exit");
}

double parse_param_value(const std::string& raw, mavlink::MAV_PARAM_TYPE type) {
  switch (type) {
    case mavlink::MAV_PARAM_TYPE_INT8:
    case mavlink::MAV_PARAM_TYPE_INT16:
    case mavlink::MAV_PARAM_TYPE_INT32:
    case mavlink::MAV_PARAM_TYPE_UINT8:
    case mavlink::MAV_PARAM_TYPE_UINT16:
    case mavlink::MAV_PARAM_TYPE_UINT32:
      return static_cast<double>(std::stol(raw));
    default:
      return std::stod(raw);
  }
}

std::vector<std::string> split_words(const std::string& line) {
  std::istringstream stream(line);
  std::vector<std::string> parts;
  std::string word;
  while (stream >> word) {
    parts.push_back(word);
  }
  return parts;
}

void cancel_operation(GcsContext& ctx) {
  if (ctx.operation_cancel == nullptr || ctx.operation_cancel->is_cancelled()) {
    std::puts("[cancel] no active cancellable operation");
    return;
  }
  ctx.operation_cancel->cancel();
  std::puts("[cancel] signalled");
}

void print_heartbeat_status(GcsContext& ctx) {
  const auto& node = ctx.vehicle;
  const bool online = ctx.heartbeat_monitor()->is_online(node);
  const auto state = ctx.heartbeat_monitor()->state_for(node);

  std::printf("[heartbeat] vehicle %u:%u online=%s\n", node.system_id, node.component_id, online ? "true" : "false");
  if (state.has_value()) {
    const auto age_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
      std::chrono::steady_clock::now() - state->received_at
    ).count();
    std::printf(
      "  last=%lldms ago type=%d status=%d\n",
      static_cast<long long>(age_ms),
      static_cast<int>(state->heartbeat_msg.type),
      static_cast<int>(state->heartbeat_msg.system_status)
    );
  } else {
    std::puts("  no heartbeat received yet");
  }
}

void fetch_all_parameters(GcsContext& ctx) {
  ctx.operation_cancel = std::make_unique<mavlink::MavlinkCancellationToken>();

  std::puts("[parameters] waiting for PARAM_VALUE stream...");
  const auto entries = ctx.parameters().fetch_all(
    [&](const mavlink::ParamEntry& entry, int received, int expected) {
      if (received == 1) {
        std::printf("[parameters] expecting %d parameters\n", expected);
      }
      std::printf(
        "[parameters] %d/%d %s=%g (type=%d)\n",
        received,
        expected,
        entry.id.c_str(),
        entry.value,
        static_cast<int>(entry.type)
      );
    },
    ctx.operation_cancel.get()
  );
  std::printf(
    "[parameters] complete (%zu total, cache=%zu)\n",
    entries.size(),
    ctx.parameters().cache().size()
  );
}

void read_parameter(GcsContext& ctx, const std::vector<std::string>& parts) {
  if (parts.size() < 2) {
    std::puts("Usage: pr <name>");
    return;
  }

  const std::string& name = parts[1];
  std::printf("[parameters] reading %s...\n", name.c_str());
  const auto entry = ctx.parameters().read_by_name(name);
  std::printf(
    "[parameters] %s=%g (type=%d, index %u/%u)\n",
    name.c_str(),
    entry.value,
    static_cast<int>(entry.type),
    entry.index,
    entry.count
  );
}

void write_parameter(GcsContext& ctx, const std::vector<std::string>& parts) {
  if (parts.size() < 3) {
    std::puts("Usage: pw <name> <value>");
    return;
  }

  const std::string& name = parts[1];
  const auto cached_type = ctx.parameters().type_for_name(name);
  const mavlink::MAV_PARAM_TYPE type =
    cached_type.has_value() ? cached_type.value() : mavlink::MAV_PARAM_TYPE_REAL32;
  const double value = parse_param_value(parts[2], type);

  std::printf("[parameters] writing %s=%g (type=%d)...\n", name.c_str(), value, static_cast<int>(type));
  const auto entry = ctx.parameters().write_by_name(name, value);
  std::printf(
    "[parameters] ack %s=%g (type=%d)\n",
    name.c_str(),
    entry.value,
    static_cast<int>(entry.type)
  );
}

void upload_mission(GcsContext& ctx) {
  const auto plan = build_sample_mission(ctx.target_system(), ctx.target_component());
  ctx.operation_cancel = std::make_unique<mavlink::MavlinkCancellationToken>();

  std::printf("[mission] uploading %zu hardcoded items...\n", plan.size());
  const auto result = ctx.mission().upload(
    plan,
    mavlink::MAV_MISSION_TYPE_MISSION,
    [&](int sent, int total, const mavlink::mission_item_int_t& item) {
      std::printf(
        "[mission upload] %d/%d %s\n",
        sent,
        total,
        describe_mission_item(item).c_str()
      );
    },
    ctx.operation_cancel.get()
  );
  std::printf("[mission] upload finished: %d\n", static_cast<int>(result));
}

void download_mission(GcsContext& ctx) {
  ctx.operation_cancel = std::make_unique<mavlink::MavlinkCancellationToken>();

  const auto items = ctx.mission().download(
    mavlink::MAV_MISSION_TYPE_MISSION,
    [&](int received, int total, const mavlink::mission_item_int_t& item) {
      std::printf(
        "[mission download] %d/%d %s\n",
        received,
        total,
        describe_mission_item(item).c_str()
      );
    },
    ctx.operation_cancel.get()
  );

  std::puts("[mission] on vehicle:");
  for (const auto& item : items) {
    std::printf("  %s\n", describe_mission_item(item).c_str());
  }
}

void clear_mission(GcsContext& ctx) {
  std::puts("[mission] sending MISSION_CLEAR_ALL...");
  const auto result = ctx.mission().clear();
  std::printf("[mission] clear result: %d\n", static_cast<int>(result));
}

void set_mission_current(GcsContext& ctx, const std::vector<std::string>& parts) {
  if (parts.size() < 2) {
    std::puts("Usage: ms <seq>");
    return;
  }

  const uint16_t seq = static_cast<uint16_t>(std::stoul(parts[1]));
  std::printf("[mission] set current seq=%u (mission + command)...\n", seq);
  const auto result = ctx.mission().set_current_with_command(seq, &ctx.command());
  std::printf(
    "[mission] seq=%u command ack=%d\n",
    result.sequence,
    result.command_ack.has_value() ? static_cast<int>(result.command_ack->result) : -1
  );
}

void request_message(GcsContext& ctx, const std::vector<std::string>& parts) {
  if (parts.size() < 2) {
    std::printf("Usage: rm <msgId>  (e.g. rm %u for ATTITUDE)\n", mavlink::attitude_MSG_ID);
    return;
  }

  const uint32_t msg_id = static_cast<uint32_t>(std::stoul(parts[1]));
  std::printf("[command] REQUEST_MESSAGE id=%u\n", msg_id);
  const auto ack = ctx.command().request_message(msg_id);
  std::printf("[command] ack: %d\n", static_cast<int>(ack.result));

  if (msg_id == mavlink::attitude_MSG_ID) {
    std::puts("[telemetry] waiting for ATTITUDE...");
    const auto attitude = ctx.session()->wait_for_message_type<mavlink::attitude_t>(
      mavlink::attitude_MSG_ID,
      mavlink::attitude_parse,
      ctx.target_system(),
      std::nullopt,
      std::chrono::seconds(5)
    );
    std::printf(
      "[telemetry] roll=%f pitch=%f yaw=%f\n",
      attitude.roll,
      attitude.pitch,
      attitude.yaw
    );
  }
}

void set_message_interval(GcsContext& ctx, const std::vector<std::string>& parts) {
  if (parts.size() < 3) {
    std::puts("Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)");
    return;
  }

  const uint32_t msg_id = static_cast<uint32_t>(std::stoul(parts[1]));
  const int32_t interval_us = static_cast<int32_t>(std::stol(parts[2]));
  std::printf("[command] SET_MESSAGE_INTERVAL id=%u interval=%d us\n", msg_id, interval_us);
  const auto ack = interval_us == 0
    ? ctx.command().stop_message_interval(msg_id)
    : ctx.command().set_message_interval(msg_id, interval_us);
  std::printf("[command] ack: %d\n", static_cast<int>(ack.result));
}

void stream_attitude(GcsContext& ctx, const std::vector<std::string>& parts) {
  const int seconds = parts.size() >= 2 ? std::stoi(parts[1]) : 5;
  std::printf("[telemetry] streaming ATTITUDE for %ds (subscribe + interval)...\n", seconds);

  ctx.command().set_message_interval(mavlink::attitude_MSG_ID, 100000);

  int count = 0;
  auto subscription = ctx.session()->listen_message(
    mavlink::attitude_MSG_ID,
    [&](const uint8_t* payload, size_t, const mavlink::frame_t&) {
      mavlink::attitude_t attitude{};
      mavlink::attitude_parse(payload, attitude);
      ++count;
      std::printf(
        "[attitude] #%d roll=%.3f pitch=%.3f yaw=%.3f\n",
        count,
        attitude.roll,
        attitude.pitch,
        attitude.yaw
      );
    },
    ctx.target_system()
  );

  std::this_thread::sleep_for(std::chrono::seconds(seconds));
  subscription.cancel();
  ctx.command().stop_message_interval(mavlink::attitude_MSG_ID);
  std::printf("[telemetry] received %d ATTITUDE messages\n", count);
}

void arm_vehicle(GcsContext& ctx, const std::vector<std::string>& parts) {
  const bool force = parts.size() >= 2 && parts[1] == "force";
  std::printf("[command] ARM%s...\n", force ? " (force)" : "");
  const auto ack = ctx.command().arm(force);
  std::printf("[command] ack: %d\n", static_cast<int>(ack.result));
}

void disarm_vehicle(GcsContext& ctx, const std::vector<std::string>& parts) {
  const bool force = parts.size() >= 2 && parts[1] == "force";
  std::printf("[command] DISARM%s...\n", force ? " (force)" : "");
  const auto ack = ctx.command().disarm(force);
  std::printf("[command] ack: %d\n", static_cast<int>(ack.result));
}

void return_to_launch(GcsContext& ctx) {
  std::puts("[command] RETURN_TO_LAUNCH...");
  const auto ack = ctx.command().return_to_launch();
  std::printf("[command] ack: %d\n", static_cast<int>(ack.result));
}

void run_cli(GcsContext& ctx) {
  print_help();

  while (true) {
    std::cout << "gcs> " << std::flush;
    std::string line;
    if (!std::getline(std::cin, line)) {
      break;
    }

    const auto trimmed = line;
    if (trimmed.empty()) {
      continue;
    }

    const auto parts = split_words(trimmed);
    const std::string& command = parts.front();

    try {
      if (command == "h" || command == "help") {
        print_help();
      } else if (command == "q" || command == "quit" || command == "exit") {
        return;
      } else if (command == "hb") {
        print_heartbeat_status(ctx);
      } else if (command == "cancel") {
        cancel_operation(ctx);
      } else if (command == "p" || command == "params") {
        fetch_all_parameters(ctx);
      } else if (command == "pr") {
        read_parameter(ctx, parts);
      } else if (command == "pw") {
        write_parameter(ctx, parts);
      } else if (command == "mu") {
        upload_mission(ctx);
      } else if (command == "md") {
        download_mission(ctx);
      } else if (command == "mc") {
        clear_mission(ctx);
      } else if (command == "ms") {
        set_mission_current(ctx, parts);
      } else if (command == "rm") {
        request_message(ctx, parts);
      } else if (command == "si") {
        set_message_interval(ctx, parts);
      } else if (command == "att") {
        stream_attitude(ctx, parts);
      } else if (command == "arm") {
        arm_vehicle(ctx, parts);
      } else if (command == "disarm") {
        disarm_vehicle(ctx, parts);
      } else if (command == "rtl") {
        return_to_launch(ctx);
      } else {
        std::printf("Unknown command: %s (type help)\n", command.c_str());
      }
    } catch (const mavlink::MavlinkCancelledException&) {
      std::puts("Operation cancelled.");
    } catch (const std::exception& error) {
      std::printf("Error: %s\n", error.what());
    }

    std::puts("");
  }
}

}  // namespace

int main(int argc, char* argv[]) {
  try {
    const int baud_rate = sitl_gcs::parse_baud_rate(argc, argv);
    const std::string port_name = sitl_gcs::pick_serial_port();

    std::printf("\nOpening %s @ %d baud...\n", port_name.c_str(), baud_rate);

    mavlink::mavlink_dialect_rt_rc_t dialect{};
    mavlink::mavlink_dialect_rt_rc_init(dialect);

    const auto link = sitl_gcs::SerialMavlinkLink::open(port_name, baud_rate);
    mavlink::MavlinkGcs gcs = mavlink::MavlinkGcs::connect(
      &dialect.base,
      link,
      sitl_gcs::kGcsSystemId,
      sitl_gcs::kGcsComponentId
    );

    gcs.start();
    std::puts("Publishing GCS heartbeats, waiting for vehicle...");

    const std::set<uint8_t> exclude = {sitl_gcs::kGcsSystemId};
    mavlink::MavlinkVehicleClient client = [&]() {
      try {
        return gcs.wait_for_vehicle(&exclude, std::chrono::seconds(60));
      } catch (const mavlink::MavlinkTimeoutException&) {
        throw std::runtime_error(
          "No vehicle heartbeat within 60 s. Check port, baud (current: " +
          std::to_string(baud_rate) + "; try --baud 115200), and SITL."
        );
      }
    }();

    const auto& vehicle = client.vehicle();
    const auto vehicle_state = gcs.heartbeat_monitor()->state_for(vehicle);
    std::printf("Vehicle online: %u:%u\n", vehicle.system_id, vehicle.component_id);
    if (vehicle_state.has_value()) {
      std::printf(
        "  type=%d autopilot=%d status=%d\n",
        static_cast<int>(vehicle_state->heartbeat_msg.type),
        static_cast<int>(vehicle_state->heartbeat_msg.autopilot),
        static_cast<int>(vehicle_state->heartbeat_msg.system_status)
      );
    }

    sitl_gcs::GcsContext ctx{gcs, vehicle, std::move(client)};

    std::puts("\n=== Phase 2: parameter sync ===");
    fetch_all_parameters(ctx);

    std::puts("\n=== Interactive CLI ===");
    run_cli(ctx);

    std::puts("Shutting down...");
    if (ctx.operation_cancel != nullptr) {
      ctx.operation_cancel->cancel();
    }
    gcs.close();
    return 0;
  } catch (const std::exception& error) {
    std::fprintf(stderr, "Fatal: %s\n", error.what());
    return 1;
  }
}
