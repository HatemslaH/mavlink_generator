#include "parameter_protocol.hpp"

#include <set>
#include <stdexcept>

namespace mavlink {

ParamEntry ParamEntry::from_param_value(const param_value_t& message) {
  char id_buf[17];
  ParamCodec::param_id_to_string(message.param_id, id_buf, sizeof(id_buf));
  return ParamEntry{
    id_buf,
    ParamCodec::decode_value(message.param_value, message.param_type),
    message.param_type,
    message.param_index,
    message.param_count,
  };
}

ParameterProtocol::ParameterProtocol(
  MavlinkSession* session,
  uint8_t target_system,
  uint8_t target_component,
  std::chrono::milliseconds idle_timeout,
  std::chrono::milliseconds request_timeout
)
    : session_(session),
      target_system_(target_system),
      target_component_(target_component),
      idle_timeout_(idle_timeout),
      request_timeout_(request_timeout) {}

const std::map<std::string, ParamEntry>& ParameterProtocol::cache() const { return cache_; }

void ParameterProtocol::clear_cache() { cache_.clear(); }

std::optional<MAV_PARAM_TYPE> ParameterProtocol::type_for_name(const std::string& name) const {
  const auto it = cache_.find(name);
  if (it == cache_.end()) {
    return std::nullopt;
  }
  return it->second.type;
}

void ParameterProtocol::remember(const ParamEntry& entry) { cache_[entry.id] = entry; }

std::vector<ParamEntry> ParameterProtocol::fetch_all(
  ParamProgressCallback on_progress,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  param_request_list_t list_request{};
  list_request.target_system = target_system_;
  list_request.target_component = target_component_;
  uint8_t payload[param_request_list_ENCODED_LENGTH];
  param_request_list_serialize(list_request, payload);
  session_->send_frame(
    param_request_list_MSG_ID,
    param_request_list_CRC_EXTRA,
    payload,
    param_request_list_ENCODED_LENGTH
  );

  std::vector<ParamEntry> entries;
  std::set<uint16_t> seen_indices;
  int expected_count = -1;

  while (true) {
    if (cancel != nullptr) {
      cancel->throw_if_cancelled();
    }

    const auto timeout = expected_count == -1 ? request_timeout_ : idle_timeout_;
    const frame_t frame = session_->wait_for_message(
      [&](uint32_t message_id, const uint8_t* frame_payload, size_t, uint8_t, uint8_t) {
        if (message_id != param_value_MSG_ID) {
          return false;
        }
        param_value_t value{};
        param_value_parse(frame_payload, value);
        return seen_indices.count(value.param_index) == 0;
      },
      target_system_,
      std::nullopt,
      timeout,
      cancel
    );

    param_value_t param_value{};
    param_value_parse(frame.payload, param_value);
    seen_indices.insert(param_value.param_index);

    if (expected_count == -1) {
      expected_count = param_value.param_count;
    }

    const ParamEntry entry = ParamEntry::from_param_value(param_value);
    remember(entry);
    entries.push_back(entry);

    if (on_progress) {
      on_progress(entry, static_cast<int>(entries.size()), entry.count);
    }

    if (seen_indices.size() >= static_cast<size_t>(expected_count)) {
      break;
    }
  }

  return entries;
}

ParamEntry ParameterProtocol::read_by_name(const std::string& name, MavlinkCancellationToken* cancel) {
  return read(name.c_str(), -1, cancel);
}

ParamEntry ParameterProtocol::read_by_index(int index, MavlinkCancellationToken* cancel) {
  return read(nullptr, index, cancel);
}

ParamEntry ParameterProtocol::read(const char* param_id, int param_index, MavlinkCancellationToken* cancel) {
  if (param_id == nullptr && param_index < 0) {
    throw std::invalid_argument("Either param_id or a non-negative param_index is required");
  }

  param_request_read_t request{};
  request.param_index = static_cast<int16_t>(param_index);
  request.target_system = target_system_;
  request.target_component = target_component_;
  ParamCodec::param_id_from_string(request.param_id, param_id != nullptr ? param_id : "");

  uint8_t payload[param_request_read_ENCODED_LENGTH];
  param_request_read_serialize(request, payload);
  session_->send_frame(
    param_request_read_MSG_ID,
    param_request_read_CRC_EXTRA,
    payload,
    param_request_read_ENCODED_LENGTH
  );

  const param_value_t value = session_->wait_for_message_type<param_value_t>(
    param_value_MSG_ID,
    param_value_parse,
    target_system_,
    std::nullopt,
    request_timeout_,
    cancel
  );

  const ParamEntry entry = ParamEntry::from_param_value(value);
  remember(entry);
  return entry;
}

ParamEntry ParameterProtocol::write(
  const std::string& name,
  double value,
  MAV_PARAM_TYPE type,
  MavlinkCancellationToken* cancel
) {
  param_set_t set_msg{};
  set_msg.param_value = ParamCodec::encode_value(value, type);
  set_msg.target_system = target_system_;
  set_msg.target_component = target_component_;
  ParamCodec::param_id_from_string(set_msg.param_id, name.c_str());
  set_msg.param_type = type;

  uint8_t payload[param_set_ENCODED_LENGTH];
  param_set_serialize(set_msg, payload);
  session_->send_frame(param_set_MSG_ID, param_set_CRC_EXTRA, payload, param_set_ENCODED_LENGTH);

  const frame_t frame = session_->wait_for_message(
    [&](uint32_t message_id, const uint8_t* frame_payload, size_t, uint8_t, uint8_t) {
      if (message_id != param_value_MSG_ID) {
        return false;
      }
      param_value_t ack{};
      param_value_parse(frame_payload, ack);
      char id_buf[17];
      ParamCodec::param_id_to_string(ack.param_id, id_buf, sizeof(id_buf));
      return name == id_buf;
    },
    target_system_,
    std::nullopt,
    request_timeout_,
    cancel
  );

  param_value_t ack{};
  param_value_parse(frame.payload, ack);
  const ParamEntry entry = ParamEntry::from_param_value(ack);
  remember(entry);
  return entry;
}

ParamEntry ParameterProtocol::write_by_name(
  const std::string& name,
  double value,
  std::optional<MAV_PARAM_TYPE> type,
  MavlinkCancellationToken* cancel
) {
  const MAV_PARAM_TYPE resolved =
    type.has_value() ? type.value() : type_for_name(name).value_or(MAV_PARAM_TYPE_REAL32);
  return write(name, value, resolved, cancel);
}

ParameterServer::ParameterServer(
  MavlinkSession* session,
  const std::map<std::string, ParamStoredValue>* initial_values
)
    : session_(session) {
  if (initial_values != nullptr) {
    values_ = *initial_values;
  }
  frame_listener_id_ = session_->add_frame_listener([this](const frame_t& frame) { on_frame(frame); });
}

ParameterServer::~ParameterServer() { close(); }

const std::map<std::string, ParamStoredValue>& ParameterServer::values() const { return values_; }

void ParameterServer::set(const std::string& name, double value, MAV_PARAM_TYPE type) {
  values_[name] = ParamStoredValue{value, type};
}

void ParameterServer::close() {
  if (closed_) {
    return;
  }
  closed_ = true;
  session_->remove_frame_listener(frame_listener_id_);
}

void ParameterServer::on_frame(const frame_t& frame) {
  if (frame.message_id == param_request_list_MSG_ID) {
    param_request_list_t message{};
    param_request_list_parse(frame.payload, message);
    if (message.target_system != session_->system_id() &&
        message.target_system != MAV_COMP_ID_ALL) {
      return;
    }
    broadcast_all();
    return;
  }

  if (frame.message_id == param_request_read_MSG_ID) {
    param_request_read_t message{};
    param_request_read_parse(frame.payload, message);
    if (message.target_system != session_->system_id() &&
        message.target_system != MAV_COMP_ID_ALL) {
      return;
    }
    const auto entry = resolve_read(message);
    if (entry.has_value()) {
      send_value(entry->first, entry->second, index_of(entry->first));
    }
    return;
  }

  if (frame.message_id == param_set_MSG_ID) {
    param_set_t message{};
    param_set_parse(frame.payload, message);
    if (message.target_system != session_->system_id()) {
      return;
    }
    char id_buf[17];
    ParamCodec::param_id_to_string(message.param_id, id_buf, sizeof(id_buf));
    values_[id_buf] = ParamStoredValue{
      ParamCodec::decode_value(message.param_value, message.param_type),
      message.param_type,
    };
    send_value(id_buf, values_[id_buf], index_of(id_buf));
  }
}

void ParameterServer::broadcast_all() {
  int index = 0;
  for (const auto& entry : values_) {
    send_value(entry.first, entry.second, index++);
  }
}

void ParameterServer::send_value(const std::string& name, const ParamStoredValue& entry, int index) {
  param_value_t value{};
  value.param_value = ParamCodec::encode_value(entry.value, entry.type);
  value.param_count = static_cast<uint16_t>(values_.size());
  value.param_index = static_cast<uint16_t>(index);
  ParamCodec::param_id_from_string(value.param_id, name.c_str());
  value.param_type = entry.type;

  uint8_t payload[param_value_ENCODED_LENGTH];
  param_value_serialize(value, payload);
  session_->send_frame(
    param_value_MSG_ID,
    param_value_CRC_EXTRA,
    payload,
    param_value_ENCODED_LENGTH
  );
}

std::optional<std::pair<std::string, ParamStoredValue>> ParameterServer::resolve_read(
  const param_request_read_t& request
) const {
  if (request.param_index >= 0) {
    int index = 0;
    for (const auto& entry : values_) {
      if (index == request.param_index) {
        return std::make_pair(entry.first, entry.second);
      }
      index++;
    }
    return std::nullopt;
  }

  char id_buf[17];
  ParamCodec::param_id_to_string(request.param_id, id_buf, sizeof(id_buf));
  const auto it = values_.find(id_buf);
  if (it == values_.end()) {
    return std::nullopt;
  }
  return std::make_pair(it->first, it->second);
}

int ParameterServer::index_of(const std::string& name) const {
  int index = 0;
  for (const auto& entry : values_) {
    if (entry.first == name) {
      return index;
    }
    index++;
  }
  return -1;
}

}  // namespace mavlink
