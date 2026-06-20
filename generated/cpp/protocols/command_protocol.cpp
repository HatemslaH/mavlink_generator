#include "command_protocol.hpp"

namespace mavlink {

CommandProtocol::CommandProtocol(
  MavlinkSession* session,
  uint8_t target_system,
  uint8_t target_component,
  std::chrono::milliseconds default_timeout
)
    : session_(session),
      target_system_(target_system),
      target_component_(target_component),
      default_timeout_(default_timeout) {}

command_ack_t CommandProtocol::send_long(
  const command_long_t& command,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  uint8_t payload[command_long_ENCODED_LENGTH];
  command_long_serialize(command, payload);
  session_->send_frame(
    command_long_MSG_ID,
    command_long_CRC_EXTRA,
    payload,
    command_long_ENCODED_LENGTH
  );
  return wait_for_ack(command.command, timeout, cancel);
}

command_ack_t CommandProtocol::send_int(
  const command_int_t& command,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  uint8_t payload[command_int_ENCODED_LENGTH];
  command_int_serialize(command, payload);
  session_->send_frame(
    command_int_MSG_ID,
    command_int_CRC_EXTRA,
    payload,
    command_int_ENCODED_LENGTH
  );
  return wait_for_ack(command.command, timeout, cancel);
}

command_ack_t CommandProtocol::command_long(
  MAV_CMD command,
  float param1,
  float param2,
  float param3,
  float param4,
  float param5,
  float param6,
  float param7,
  uint8_t confirmation,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  command_long_t message{};
  message.param1 = param1;
  message.param2 = param2;
  message.param3 = param3;
  message.param4 = param4;
  message.param5 = param5;
  message.param6 = param6;
  message.param7 = param7;
  message.command = command;
  message.target_system = target_system_;
  message.target_component = target_component_;
  message.confirmation = confirmation;
  return send_long(message, timeout, cancel);
}

command_ack_t CommandProtocol::request_message(
  uint32_t message_id,
  float param2,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_REQUEST_MESSAGE,
    static_cast<float>(message_id),
    param2,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::set_message_interval(
  uint32_t message_id,
  int32_t interval_us,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_SET_MESSAGE_INTERVAL,
    static_cast<float>(message_id),
    static_cast<float>(interval_us),
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::stop_message_interval(
  uint32_t message_id,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return set_message_interval(message_id, 0, timeout, cancel);
}

command_ack_t CommandProtocol::set_mission_current(
  uint16_t sequence,
  bool reset_mission,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_DO_SET_MISSION_CURRENT,
    static_cast<float>(sequence),
    reset_mission ? 1.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::arm(
  bool force,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_COMPONENT_ARM_DISARM,
    1,
    force ? 21196.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::disarm(
  bool force,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_COMPONENT_ARM_DISARM,
    0,
    force ? 21196.0f : 0.0f,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::takeoff(
  double altitude,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_NAV_TAKEOFF,
    0,
    0,
    0,
    0,
    0,
    0,
    static_cast<float>(altitude),
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::land(
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(MAV_CMD_NAV_LAND, 0, 0, 0, 0, 0, 0, 0, 0, timeout, cancel);
}

command_ack_t CommandProtocol::return_to_launch(
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  return command_long(
    MAV_CMD_NAV_RETURN_TO_LAUNCH,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    timeout,
    cancel
  );
}

command_ack_t CommandProtocol::wait_for_ack(
  MAV_CMD command,
  std::optional<std::chrono::milliseconds> timeout,
  MavlinkCancellationToken* cancel
) {
  const auto resolved_timeout = timeout.value_or(default_timeout_);
  const frame_t frame = session_->wait_for_message(
    [command](uint32_t message_id, const uint8_t* payload, size_t, uint8_t, uint8_t) {
      if (message_id != command_ack_MSG_ID) {
        return false;
      }
      command_ack_t ack{};
      command_ack_parse(payload, ack);
      return ack.command == command;
    },
    target_system_,
    std::nullopt,
    resolved_timeout,
    cancel
  );

  command_ack_t ack{};
  command_ack_parse(frame.payload, ack);
  return ack;
}

CommandServer::CommandServer(
  MavlinkSession* session,
  CommandLongHandler on_command_long,
  CommandIntHandler on_command_int
)
    : session_(session),
      on_command_long_(std::move(on_command_long)),
      on_command_int_(std::move(on_command_int)) {
  frame_listener_id_ = session_->add_frame_listener([this](const frame_t& frame) { on_frame(frame); });
}

CommandServer::~CommandServer() { close(); }

void CommandServer::close() {
  if (closed_) {
    return;
  }
  closed_ = true;
  session_->remove_frame_listener(frame_listener_id_);
}

void CommandServer::on_frame(const frame_t& frame) {
  if (frame.message_id == command_long_MSG_ID) {
    command_long_t message{};
    command_long_parse(frame.payload, message);
    if (message.target_system != session_->system_id()) {
      return;
    }
    const MAV_RESULT result =
      on_command_long_ ? on_command_long_(message) : MAV_RESULT_ACCEPTED;
    send_ack(frame, message.command, result);
    return;
  }

  if (frame.message_id == command_int_MSG_ID) {
    command_int_t message{};
    command_int_parse(frame.payload, message);
    if (message.target_system != session_->system_id()) {
      return;
    }
    const MAV_RESULT result = on_command_int_ ? on_command_int_(message) : MAV_RESULT_ACCEPTED;
    send_ack(frame, message.command, result);
  }
}

void CommandServer::send_ack(const frame_t& request_frame, MAV_CMD command, MAV_RESULT result) {
  command_ack_t ack{};
  ack.command = command;
  ack.result = result;
  ack.progress = 0;
  ack.result_param2 = 0;
  ack.target_system = request_frame.system_id;
  ack.target_component = request_frame.component_id;

  uint8_t payload[command_ack_ENCODED_LENGTH];
  command_ack_serialize(ack, payload);
  session_->send_frame(
    command_ack_MSG_ID,
    command_ack_CRC_EXTRA,
    payload,
    command_ack_ENCODED_LENGTH
  );
}

}  // namespace mavlink
