#pragma once

#include <chrono>
#include <cstdint>
#include <functional>
#include <optional>

#include "../mavlink.hpp"
#include "mavlink_cancellation.hpp"
#include "mavlink_session.hpp"

namespace mavlink {

/// GCS-side MAVLink command protocol client.
class CommandProtocol {
 public:
  CommandProtocol(
    MavlinkSession* session,
    uint8_t target_system,
    uint8_t target_component,
    std::chrono::milliseconds default_timeout = std::chrono::seconds(5)
  );

  command_ack_t send_long(
    const command_long_t& command,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t send_int(
    const command_int_t& command,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t command_long(
    MAV_CMD command,
    float param1 = 0,
    float param2 = 0,
    float param3 = 0,
    float param4 = 0,
    float param5 = 0,
    float param6 = 0,
    float param7 = 0,
    uint8_t confirmation = 0,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t request_message(
    uint32_t message_id,
    float param2 = 0,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t set_message_interval(
    uint32_t message_id,
    int32_t interval_us,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t stop_message_interval(
    uint32_t message_id,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t set_mission_current(
    uint16_t sequence,
    bool reset_mission = false,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t arm(
    bool force = false,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t disarm(
    bool force = false,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t takeoff(
    double altitude = 10,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t land(
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t return_to_launch(
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

  command_ack_t wait_for_ack(
    MAV_CMD command,
    std::optional<std::chrono::milliseconds> timeout = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

 private:
  MavlinkSession* session_;
  uint8_t target_system_;
  uint8_t target_component_;
  std::chrono::milliseconds default_timeout_;
};

using CommandLongHandler = std::function<MAV_RESULT(const command_long_t&)>;
using CommandIntHandler = std::function<MAV_RESULT(const command_int_t&)>;

/// Vehicle-side command handler.
class CommandServer {
 public:
  CommandServer(
    MavlinkSession* session,
    CommandLongHandler on_command_long = nullptr,
    CommandIntHandler on_command_int = nullptr
  );

  ~CommandServer();

  void close();

 private:
  void on_frame(const frame_t& frame);
  void send_ack(const frame_t& request_frame, MAV_CMD command, MAV_RESULT result);

  MavlinkSession* session_;
  CommandLongHandler on_command_long_;
  CommandIntHandler on_command_int_;
  size_t frame_listener_id_ = 0;
  bool closed_ = false;
};

}  // namespace mavlink
