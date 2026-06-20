#include "mission_protocol.hpp"

namespace mavlink {

mission_item_int_t MissionItems::waypoint(
  uint16_t seq,
  double latitude,
  double longitude,
  float altitude,
  uint8_t target_system,
  uint8_t target_component,
  MAV_CMD command,
  MAV_FRAME frame,
  MAV_MISSION_TYPE mission_type,
  float param1,
  float param2,
  float param3,
  float param4,
  uint8_t current,
  uint8_t autocontinue
) {
  mission_item_int_t item{};
  item.param1 = param1;
  item.param2 = param2;
  item.param3 = param3;
  item.param4 = param4;
  item.x = static_cast<int32_t>(latitude * 1e7);
  item.y = static_cast<int32_t>(longitude * 1e7);
  item.z = altitude;
  item.seq = seq;
  item.command = command;
  item.target_system = target_system;
  item.target_component = target_component;
  item.frame = frame;
  item.current = current;
  item.autocontinue = autocontinue;
  item.mission_type = mission_type;
  return item;
}

mission_item_t MissionItems::to_legacy_item(const mission_item_int_t& item) {
  mission_item_t legacy{};
  legacy.param1 = item.param1;
  legacy.param2 = item.param2;
  legacy.param3 = item.param3;
  legacy.param4 = item.param4;
  legacy.x = static_cast<float>(item.x) / 1e7f;
  legacy.y = static_cast<float>(item.y) / 1e7f;
  legacy.z = item.z;
  legacy.seq = item.seq;
  legacy.command = item.command;
  legacy.target_system = item.target_system;
  legacy.target_component = item.target_component;
  legacy.frame = item.frame;
  legacy.current = item.current;
  legacy.autocontinue = item.autocontinue;
  legacy.mission_type = item.mission_type;
  return legacy;
}

mission_item_int_t MissionItems::from_legacy_item(const mission_item_t& item) {
  mission_item_int_t modern{};
  modern.param1 = item.param1;
  modern.param2 = item.param2;
  modern.param3 = item.param3;
  modern.param4 = item.param4;
  modern.x = static_cast<int32_t>(item.x * 1e7f);
  modern.y = static_cast<int32_t>(item.y * 1e7f);
  modern.z = item.z;
  modern.seq = item.seq;
  modern.command = item.command;
  modern.target_system = item.target_system;
  modern.target_component = item.target_component;
  modern.frame = item.frame;
  modern.current = item.current;
  modern.autocontinue = item.autocontinue;
  modern.mission_type = item.mission_type;
  return modern;
}

std::vector<mission_item_int_t> MissionItems::with_sequential_seq(
  const std::vector<mission_item_int_t>& items
) {
  std::vector<mission_item_int_t> result;
  result.reserve(items.size());
  for (size_t index = 0; index < items.size(); index++) {
    mission_item_int_t item = items[index];
    item.seq = static_cast<uint16_t>(index);
    result.push_back(item);
  }
  return result;
}

MissionProtocol::MissionProtocol(
  MavlinkSession* session,
  uint8_t target_system,
  uint8_t target_component,
  std::chrono::milliseconds item_timeout,
  std::chrono::milliseconds operation_timeout
)
    : session_(session),
      target_system_(target_system),
      target_component_(target_component),
      item_timeout_(item_timeout),
      operation_timeout_(operation_timeout) {}

MAV_MISSION_RESULT MissionProtocol::upload(
  const std::vector<mission_item_int_t>& items,
  MAV_MISSION_TYPE mission_type,
  MissionUploadProgressCallback on_progress,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  const auto plan = MissionItems::with_sequential_seq(items);

  mission_count_t count{};
  count.count = static_cast<uint16_t>(plan.size());
  count.target_system = target_system_;
  count.target_component = target_component_;
  count.mission_type = mission_type;
  uint8_t count_payload[mission_count_ENCODED_LENGTH];
  mission_count_serialize(count, count_payload);
  session_->send_frame(
    mission_count_MSG_ID,
    mission_count_CRC_EXTRA,
    count_payload,
    mission_count_ENCODED_LENGTH
  );

  for (const auto& item : plan) {
    if (cancel != nullptr) {
      cancel->throw_if_cancelled();
    }

    const frame_t request_frame = session_->wait_for_message(
      [&](uint32_t message_id, const uint8_t* payload, size_t, uint8_t, uint8_t) {
        return is_item_request(message_id, payload, item.seq, mission_type);
      },
      target_system_,
      std::nullopt,
      item_timeout_,
      cancel
    );

    if (request_frame.message_id == mission_request_int_MSG_ID) {
      uint8_t payload[mission_item_int_ENCODED_LENGTH];
      mission_item_int_serialize(item, payload);
      session_->send_frame(
        mission_item_int_MSG_ID,
        mission_item_int_CRC_EXTRA,
        payload,
        mission_item_int_ENCODED_LENGTH
      );
    } else if (request_frame.message_id == mission_request_MSG_ID) {
      const mission_item_t legacy = MissionItems::to_legacy_item(item);
      uint8_t payload[mission_item_ENCODED_LENGTH];
      mission_item_serialize(legacy, payload);
      session_->send_frame(
        mission_item_MSG_ID,
        mission_item_CRC_EXTRA,
        payload,
        mission_item_ENCODED_LENGTH
      );
    }

    if (on_progress) {
      on_progress(static_cast<int>(item.seq + 1), static_cast<int>(plan.size()), item);
    }
  }

  const mission_ack_t ack = session_->wait_for_message_type<mission_ack_t>(
    mission_ack_MSG_ID,
    mission_ack_parse,
    target_system_,
    std::nullopt,
    operation_timeout_,
    cancel
  );

  return ack.type;
}

std::vector<mission_item_int_t> MissionProtocol::download(
  MAV_MISSION_TYPE mission_type,
  MissionDownloadProgressCallback on_progress,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  mission_request_list_t list{};
  list.target_system = target_system_;
  list.target_component = target_component_;
  list.mission_type = mission_type;
  uint8_t list_payload[mission_request_list_ENCODED_LENGTH];
  mission_request_list_serialize(list, list_payload);
  session_->send_frame(
    mission_request_list_MSG_ID,
    mission_request_list_CRC_EXTRA,
    list_payload,
    mission_request_list_ENCODED_LENGTH
  );

  const mission_count_t count_message = session_->wait_for_message_type<mission_count_t>(
    mission_count_MSG_ID,
    mission_count_parse,
    target_system_,
    std::nullopt,
    operation_timeout_,
    cancel
  );

  std::vector<mission_item_int_t> items;

  for (uint16_t seq = 0; seq < count_message.count; seq++) {
    if (cancel != nullptr) {
      cancel->throw_if_cancelled();
    }

    mission_request_int_t request{};
    request.seq = seq;
    request.target_system = target_system_;
    request.target_component = target_component_;
    request.mission_type = mission_type;
    uint8_t request_payload[mission_request_int_ENCODED_LENGTH];
    mission_request_int_serialize(request, request_payload);
    session_->send_frame(
      mission_request_int_MSG_ID,
      mission_request_int_CRC_EXTRA,
      request_payload,
      mission_request_int_ENCODED_LENGTH
    );

    const frame_t item_frame = session_->wait_for_message(
      [seq, mission_type](uint32_t message_id, const uint8_t* payload, size_t, uint8_t, uint8_t) {
        if (message_id == mission_item_int_MSG_ID) {
          mission_item_int_t item{};
          mission_item_int_parse(payload, item);
          return item.seq == seq && item.mission_type == mission_type;
        }
        if (message_id == mission_item_MSG_ID) {
          mission_item_t item{};
          mission_item_parse(payload, item);
          return item.seq == seq && item.mission_type == mission_type;
        }
        return false;
      },
      target_system_,
      std::nullopt,
      item_timeout_,
      cancel
    );

    mission_item_int_t item{};
    if (item_frame.message_id == mission_item_int_MSG_ID) {
      mission_item_int_parse(item_frame.payload, item);
    } else {
      mission_item_t legacy{};
      mission_item_parse(item_frame.payload, legacy);
      item = MissionItems::from_legacy_item(legacy);
    }

    items.push_back(item);
    if (on_progress) {
      on_progress(static_cast<int>(items.size()), count_message.count, item);
    }
  }

  mission_ack_t ack{};
  ack.target_system = target_system_;
  ack.target_component = target_component_;
  ack.type = MAV_MISSION_ACCEPTED;
  ack.mission_type = mission_type;
  uint8_t ack_payload[mission_ack_ENCODED_LENGTH];
  mission_ack_serialize(ack, ack_payload);
  session_->send_frame(
    mission_ack_MSG_ID,
    mission_ack_CRC_EXTRA,
    ack_payload,
    mission_ack_ENCODED_LENGTH
  );

  return items;
}

MAV_MISSION_RESULT MissionProtocol::clear(
  MAV_MISSION_TYPE mission_type,
  MavlinkCancellationToken* cancel
) {
  mission_clear_all_t clear_msg{};
  clear_msg.target_system = target_system_;
  clear_msg.target_component = target_component_;
  clear_msg.mission_type = mission_type;
  uint8_t payload[mission_clear_all_ENCODED_LENGTH];
  mission_clear_all_serialize(clear_msg, payload);
  session_->send_frame(
    mission_clear_all_MSG_ID,
    mission_clear_all_CRC_EXTRA,
    payload,
    mission_clear_all_ENCODED_LENGTH
  );

  const mission_ack_t ack = session_->wait_for_message_type<mission_ack_t>(
    mission_ack_MSG_ID,
    mission_ack_parse,
    target_system_,
    std::nullopt,
    operation_timeout_,
    cancel
  );

  return ack.type;
}

void MissionProtocol::set_current(uint16_t seq, MavlinkCancellationToken* cancel) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  mission_set_current_t message{};
  message.seq = seq;
  message.target_system = target_system_;
  message.target_component = target_component_;
  uint8_t payload[mission_set_current_ENCODED_LENGTH];
  mission_set_current_serialize(message, payload);
  session_->send_frame(
    mission_set_current_MSG_ID,
    mission_set_current_CRC_EXTRA,
    payload,
    mission_set_current_ENCODED_LENGTH
  );
}

MissionSetCurrentResult MissionProtocol::set_current_with_command(
  uint16_t seq,
  CommandProtocol* command,
  bool also_send_command,
  bool reset_mission,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  set_current(seq, cancel);

  MissionSetCurrentResult result{};
  result.sequence = seq;

  if (also_send_command && command != nullptr) {
    result.command_ack = command->set_mission_current(seq, reset_mission, std::nullopt, cancel);
  }

  return result;
}

bool MissionProtocol::is_item_request(
  uint32_t message_id,
  const uint8_t* payload,
  uint16_t seq,
  MAV_MISSION_TYPE mission_type
) const {
  if (message_id == mission_request_int_MSG_ID) {
    mission_request_int_t request{};
    mission_request_int_parse(payload, request);
    return request.seq == seq && request.mission_type == mission_type;
  }
  if (message_id == mission_request_MSG_ID) {
    mission_request_t request{};
    mission_request_parse(payload, request);
    return request.seq == seq && request.mission_type == mission_type;
  }
  return false;
}

MissionServer::MissionServer(
  MavlinkSession* session,
  const std::vector<mission_item_int_t>* initial_mission,
  MAV_MISSION_TYPE mission_type
)
    : session_(session), mission_type_(mission_type) {
  if (initial_mission != nullptr) {
    items_ = *initial_mission;
  }
  frame_listener_id_ = session_->add_frame_listener([this](const frame_t& frame) { on_frame(frame); });
}

MissionServer::~MissionServer() { close(); }

const std::vector<mission_item_int_t>& MissionServer::items() const { return items_; }

void MissionServer::replace_mission(const std::vector<mission_item_int_t>& items) {
  items_ = MissionItems::with_sequential_seq(items);
  incoming_.clear();
  incoming_count_.reset();
}

void MissionServer::close() {
  if (closed_) {
    return;
  }
  closed_ = true;
  session_->remove_frame_listener(frame_listener_id_);
}

bool MissionServer::matches_target(uint8_t target_system, uint8_t target_component) const {
  if (target_system != session_->system_id() && target_system != 0) {
    return false;
  }
  if (target_component != session_->component_id() && target_component != MAV_COMP_ID_ALL) {
    return false;
  }
  return true;
}

bool MissionServer::targets_us(uint32_t message_id, const uint8_t* payload) const {
  if (message_id == mission_count_MSG_ID) {
    mission_count_t message{};
    mission_count_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_item_int_MSG_ID) {
    mission_item_int_t message{};
    mission_item_int_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_item_MSG_ID) {
    mission_item_t message{};
    mission_item_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_request_int_MSG_ID) {
    mission_request_int_t message{};
    mission_request_int_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_request_MSG_ID) {
    mission_request_t message{};
    mission_request_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_request_list_MSG_ID) {
    mission_request_list_t message{};
    mission_request_list_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  if (message_id == mission_clear_all_MSG_ID) {
    mission_clear_all_t message{};
    mission_clear_all_parse(payload, message);
    return matches_target(message.target_system, message.target_component);
  }
  return false;
}

void MissionServer::on_frame(const frame_t& frame) {
  if (!targets_us(frame.message_id, frame.payload)) {
    return;
  }

  if (frame.message_id == mission_count_MSG_ID) {
    mission_count_t message{};
    mission_count_parse(frame.payload, message);
    if (message.mission_type != mission_type_) {
      return;
    }
    incoming_count_ = message.count;
    incoming_.clear();
    if (message.count > 0) {
      request_upload_item(frame, 0);
    } else {
      send_upload_ack(frame);
    }
    return;
  }

  if (frame.message_id == mission_item_int_MSG_ID) {
    mission_item_int_t message{};
    mission_item_int_parse(frame.payload, message);
    if (message.mission_type != mission_type_) {
      return;
    }
    store_incoming_item(frame, message);
    return;
  }

  if (frame.message_id == mission_item_MSG_ID) {
    mission_item_t message{};
    mission_item_parse(frame.payload, message);
    if (message.mission_type != mission_type_) {
      return;
    }
    store_incoming_item(frame, MissionItems::from_legacy_item(message));
    return;
  }

  if (frame.message_id == mission_request_int_MSG_ID) {
    mission_request_int_t message{};
    mission_request_int_parse(frame.payload, message);
    send_requested_item(frame, message.seq);
    return;
  }

  if (frame.message_id == mission_request_MSG_ID) {
    mission_request_t message{};
    mission_request_parse(frame.payload, message);
    send_requested_item(frame, message.seq);
    return;
  }

  if (frame.message_id == mission_request_list_MSG_ID) {
    mission_request_list_t message{};
    mission_request_list_parse(frame.payload, message);
    if (message.mission_type != mission_type_) {
      return;
    }
    mission_count_t count{};
    count.count = static_cast<uint16_t>(items_.size());
    count.target_system = frame.system_id;
    count.target_component = frame.component_id;
    count.mission_type = mission_type_;
    uint8_t payload[mission_count_ENCODED_LENGTH];
    mission_count_serialize(count, payload);
    session_->send_frame(
      mission_count_MSG_ID,
      mission_count_CRC_EXTRA,
      payload,
      mission_count_ENCODED_LENGTH
    );
    return;
  }

  if (frame.message_id == mission_clear_all_MSG_ID) {
    mission_clear_all_t message{};
    mission_clear_all_parse(frame.payload, message);
    if (message.mission_type != mission_type_) {
      return;
    }
    items_.clear();
    incoming_.clear();
    incoming_count_.reset();

    mission_ack_t ack{};
    ack.target_system = frame.system_id;
    ack.target_component = frame.component_id;
    ack.type = MAV_MISSION_ACCEPTED;
    ack.mission_type = mission_type_;
    uint8_t payload[mission_ack_ENCODED_LENGTH];
    mission_ack_serialize(ack, payload);
    session_->send_frame(
      mission_ack_MSG_ID,
      mission_ack_CRC_EXTRA,
      payload,
      mission_ack_ENCODED_LENGTH
    );
  }
}

void MissionServer::store_incoming_item(const frame_t& request_frame, const mission_item_int_t& item) {
  incoming_[item.seq] = item;
  if (!incoming_count_.has_value()) {
    return;
  }

  const uint16_t expected = incoming_count_.value();
  if (incoming_.size() < expected) {
    request_upload_item(request_frame, static_cast<uint16_t>(item.seq + 1));
    return;
  }

  items_.clear();
  for (uint16_t index = 0; index < expected; index++) {
    items_.push_back(incoming_.at(index));
  }
  incoming_.clear();
  incoming_count_.reset();
  send_upload_ack(request_frame);
}

void MissionServer::request_upload_item(const frame_t& request_frame, uint16_t seq) {
  mission_request_int_t request{};
  request.seq = seq;
  request.target_system = request_frame.system_id;
  request.target_component = request_frame.component_id;
  request.mission_type = mission_type_;
  uint8_t payload[mission_request_int_ENCODED_LENGTH];
  mission_request_int_serialize(request, payload);
  session_->send_frame(
    mission_request_int_MSG_ID,
    mission_request_int_CRC_EXTRA,
    payload,
    mission_request_int_ENCODED_LENGTH
  );
}

void MissionServer::send_upload_ack(const frame_t& request_frame) {
  mission_ack_t ack{};
  ack.target_system = request_frame.system_id;
  ack.target_component = request_frame.component_id;
  ack.type = MAV_MISSION_ACCEPTED;
  ack.mission_type = mission_type_;
  uint8_t payload[mission_ack_ENCODED_LENGTH];
  mission_ack_serialize(ack, payload);
  session_->send_frame(
    mission_ack_MSG_ID,
    mission_ack_CRC_EXTRA,
    payload,
    mission_ack_ENCODED_LENGTH
  );
}

void MissionServer::send_requested_item(const frame_t& request_frame, uint16_t seq) {
  if (seq >= items_.size()) {
    mission_ack_t ack{};
    ack.target_system = request_frame.system_id;
    ack.target_component = request_frame.component_id;
    ack.type = MAV_MISSION_INVALID_SEQUENCE;
    ack.mission_type = mission_type_;
    uint8_t payload[mission_ack_ENCODED_LENGTH];
    mission_ack_serialize(ack, payload);
    session_->send_frame(
      mission_ack_MSG_ID,
      mission_ack_CRC_EXTRA,
      payload,
      mission_ack_ENCODED_LENGTH
    );
    return;
  }

  const mission_item_int_t& item = items_[seq];
  uint8_t payload[mission_item_int_ENCODED_LENGTH];
  mission_item_int_serialize(item, payload);
  session_->send_frame(
    mission_item_int_MSG_ID,
    mission_item_int_CRC_EXTRA,
    payload,
    mission_item_int_ENCODED_LENGTH
  );
}

}  // namespace mavlink
