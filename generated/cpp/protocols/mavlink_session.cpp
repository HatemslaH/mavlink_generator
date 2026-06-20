#include "mavlink_session.hpp"

#include "../mavlink_memory.hpp"
#include "../mavlink_parser.hpp"

#include <algorithm>
#include <stdexcept>

namespace mavlink {

namespace {

constexpr uint8_t kMavlinkIflagSigned = 0x01;
constexpr int kMavlinkSignatureLength = 13;
constexpr size_t kRecentFrameCapacity = 64;

void reset_parser_context(parser_t& parser, const dialect_t* dialect) {
  parser.state = parser_state_t::INIT;
  parser.version = MAVLINK_VERSION_V1;
  parser.payload_length = 0;
  parser.incompatibility_flags = 0;
  parser.compatibility_flags = 0;
  parser.sequence = 0;
  parser.system_id = 0;
  parser.component_id = 0;
  parser.message_id_low = 0;
  parser.message_id_middle = 0;
  parser.message_id_high = 0;
  parser.message_id = 0;
  parser.payload_cursor = 0;
  parser.crc_low_byte = 0;
  parser.crc_high_byte = 0;
  parser.dialect = dialect;
}

bool frame_matches_origin(
  const frame_t& frame,
  std::optional<uint8_t> from_system_id,
  std::optional<uint8_t> from_component_id
) {
  if (from_system_id.has_value() && frame.system_id != from_system_id.value()) {
    return false;
  }
  if (from_component_id.has_value() && frame.component_id != from_component_id.value()) {
    return false;
  }
  return true;
}

}  // namespace

MavlinkSession::MavlinkSession(
  const dialect_t* dialect,
  std::shared_ptr<MavlinkLink> link,
  uint8_t system_id,
  uint8_t component_id,
  version_t version
)
    : dialect_(dialect),
      link_(std::move(link)),
      system_id_(system_id),
      component_id_(component_id),
      version_(version) {
  mavlink_parser_init(parser_, dialect_);
  link_->set_receive_handler([this](const uint8_t* data, size_t len) { on_link_data(data, len); });
}

MavlinkSession::~MavlinkSession() { close(); }

void MavlinkSession::send_frame(
  uint32_t message_id,
  uint8_t crc_extra,
  const uint8_t* payload,
  size_t payload_len
) {
  std::lock_guard<std::mutex> lock(mutex_);
  if (closed_) {
    throw std::runtime_error("MavlinkSession is closed");
  }

  frame_t frame{};
  mavlink_frame_init_v2(
    frame,
    sequence_++,
    system_id_,
    component_id_,
    message_id,
    crc_extra,
    payload,
    payload_len
  );
  if (version_ == MAVLINK_VERSION_V1) {
    frame.version = MAVLINK_VERSION_V1;
  }

  uint8_t wire[MAVLINK_MAX_FRAME_SIZE];
  size_t wire_len = mavlink_frame_serialize_v2(frame, wire, sizeof(wire));
  if (wire_len == 0) {
    throw std::runtime_error("Failed to serialize MAVLink frame");
  }
  link_->send(wire, wire_len);
}

size_t MavlinkSession::add_frame_listener(frame_listener_t listener) {
  std::lock_guard<std::mutex> lock(mutex_);
  const size_t id = next_listener_id_++;
  frame_listeners_.emplace_back(id, std::move(listener));
  return id;
}

void MavlinkSession::remove_frame_listener(size_t listener_id) {
  std::lock_guard<std::mutex> lock(mutex_);
  frame_listeners_.erase(
    std::remove_if(
      frame_listeners_.begin(),
      frame_listeners_.end(),
      [listener_id](const auto& entry) { return entry.first == listener_id; }
    ),
    frame_listeners_.end()
  );
}

MavlinkMessageSubscription MavlinkSession::listen_message(
  uint32_t message_id,
  std::function<void(const uint8_t* payload, size_t payload_len, const frame_t& frame)> on_data,
  std::optional<uint8_t> from_system_id,
  std::optional<uint8_t> from_component_id
) {
  auto listener = std::make_shared<MessageListener>();
  listener->message_id = message_id;
  listener->on_data = std::move(on_data);
  listener->from_system_id = from_system_id;
  listener->from_component_id = from_component_id;

  {
    std::lock_guard<std::mutex> lock(mutex_);
    message_listeners_.push_back(listener);
  }

  return MavlinkMessageSubscription(
    [listener]() {
      listener->active = false;
      listener->on_data = nullptr;
    },
    true
  );
}

frame_t MavlinkSession::wait_for_frame(
  frame_predicate_t predicate,
  std::chrono::milliseconds timeout,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  auto pending = std::make_shared<PendingWait>();
  pending->predicate = std::move(predicate);

  {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) {
      throw std::runtime_error("MavlinkSession is closed");
    }

    frame_t recent{};
    if (check_recent_frames(pending->predicate, recent)) {
      return recent;
    }

    pending_waits_.push_back(pending);
  }

  if (cancel != nullptr) {
    cancel->on_cancel([this, pending]() {
      std::lock_guard<std::mutex> lock(mutex_);
      if (!pending->completed) {
        pending->completed = true;
        pending->success = false;
        pending->cv.notify_all();
      }
    });
    if (cancel->is_cancelled()) {
      std::lock_guard<std::mutex> lock(mutex_);
      pending_waits_.erase(
        std::remove(pending_waits_.begin(), pending_waits_.end(), pending),
        pending_waits_.end()
      );
      throw MavlinkCancelledException();
    }
  }

  std::unique_lock<std::mutex> lock(mutex_);
  const bool signaled = pending->cv.wait_for(lock, timeout, [&]() { return pending->completed; });
  pending_waits_.erase(
    std::remove(pending_waits_.begin(), pending_waits_.end(), pending),
    pending_waits_.end()
  );

  if (!signaled || !pending->success) {
    if (cancel != nullptr && cancel->is_cancelled()) {
      throw MavlinkCancelledException();
    }
    throw MavlinkTimeoutException("Timed out waiting for frame", timeout);
  }

  return pending->result;
}

frame_t MavlinkSession::wait_for_message(
  message_predicate_t predicate,
  std::optional<uint8_t> from_system_id,
  std::optional<uint8_t> from_component_id,
  std::chrono::milliseconds timeout,
  MavlinkCancellationToken* cancel
) {
  return wait_for_frame(
    [&](const frame_t& frame) {
      if (!frame_matches_origin(frame, from_system_id, from_component_id)) {
        return false;
      }
      return predicate(
        frame.message_id,
        frame.payload,
        frame.payload_len,
        frame.system_id,
        frame.component_id
      );
    },
    timeout,
    cancel
  );
}

frame_t MavlinkSession::wait_for_message_id(
  uint32_t message_id,
  std::optional<uint8_t> from_system_id,
  std::optional<uint8_t> from_component_id,
  std::chrono::milliseconds timeout,
  MavlinkCancellationToken* cancel
) {
  return wait_for_message(
    [message_id](uint32_t id, const uint8_t*, size_t, uint8_t, uint8_t) { return id == message_id; },
    from_system_id,
    from_component_id,
    timeout,
    cancel
  );
}

void MavlinkSession::close() {
  std::vector<std::shared_ptr<PendingWait>> waits;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) {
      return;
    }
    closed_ = true;
    waits = pending_waits_;
    pending_waits_.clear();
    message_listeners_.clear();
    frame_listeners_.clear();
  }

  for (auto& pending : waits) {
    pending->completed = true;
    pending->success = false;
    pending->cv.notify_all();
  }

  link_->close();
}

void MavlinkSession::on_link_data(const uint8_t* data, size_t len) {
  for (size_t i = 0; i < len; i++) {
    feed_byte(data[i]);
  }
}

void MavlinkSession::feed_byte(uint8_t byte) {
  parser_t& parser = parser_;
  static thread_local int signature_bytes_remaining = 0;

  if (signature_bytes_remaining > 0) {
    signature_bytes_remaining--;
    if (signature_bytes_remaining == 0) {
      reset_parser_context(parser, dialect_);
    }
    return;
  }

  switch (parser.state) {
    case parser_state_t::INIT:
      if (byte == MAVLINK_STX_V1) {
        parser.version = MAVLINK_VERSION_V1;
        parser.state = parser_state_t::WAIT_PAYLOAD_LENGTH;
      } else if (byte == MAVLINK_STX_V2) {
        parser.version = MAVLINK_VERSION_V2;
        parser.state = parser_state_t::WAIT_PAYLOAD_LENGTH;
      }
      break;
    case parser_state_t::WAIT_PAYLOAD_LENGTH:
      parser.payload_length = byte;
      parser.state = parser.version == MAVLINK_VERSION_V1
        ? parser_state_t::WAIT_PACKET_SEQUENCE
        : parser_state_t::WAIT_INCOMPATIBILITY_FLAGS;
      break;
    case parser_state_t::WAIT_INCOMPATIBILITY_FLAGS:
      parser.incompatibility_flags = byte;
      parser.state = parser_state_t::WAIT_COMPATIBILITY_FLAGS;
      break;
    case parser_state_t::WAIT_COMPATIBILITY_FLAGS:
      parser.compatibility_flags = byte;
      parser.state = parser_state_t::WAIT_PACKET_SEQUENCE;
      break;
    case parser_state_t::WAIT_PACKET_SEQUENCE:
      parser.sequence = byte;
      parser.state = parser_state_t::WAIT_SYSTEM_ID;
      break;
    case parser_state_t::WAIT_SYSTEM_ID:
      parser.system_id = byte;
      parser.state = parser_state_t::WAIT_COMPONENT_ID;
      break;
    case parser_state_t::WAIT_COMPONENT_ID:
      parser.component_id = byte;
      parser.state = parser.version == MAVLINK_VERSION_V1
        ? parser_state_t::WAIT_MESSAGE_ID_HIGH
        : parser_state_t::WAIT_MESSAGE_ID_LOW;
      break;
    case parser_state_t::WAIT_MESSAGE_ID_LOW:
      parser.message_id_low = byte;
      parser.state = parser_state_t::WAIT_MESSAGE_ID_MIDDLE;
      break;
    case parser_state_t::WAIT_MESSAGE_ID_MIDDLE:
      parser.message_id_middle = byte;
      parser.state = parser_state_t::WAIT_MESSAGE_ID_HIGH;
      break;
    case parser_state_t::WAIT_MESSAGE_ID_HIGH:
      if (parser.version == MAVLINK_VERSION_V1) {
        parser.message_id = byte;
      } else {
        parser.message_id_high = byte;
        parser.message_id = (static_cast<uint32_t>(parser.message_id_high) << 16) ^
                              (static_cast<uint32_t>(parser.message_id_middle) << 8) ^
                              parser.message_id_low;
      }
      if (parser.payload_length == 0) {
        parser.state = parser_state_t::WAIT_CRC_LOW_BYTE;
      } else {
        parser.payload_cursor = 0;
        parser.state = parser_state_t::WAIT_PAYLOAD_END;
      }
      break;
    case parser_state_t::WAIT_PAYLOAD_END:
      if (parser.payload_cursor < parser.payload_length) {
        parser.payload[parser.payload_cursor++] = byte;
      }
      if (parser.payload_cursor == parser.payload_length) {
        parser.state = parser_state_t::WAIT_CRC_LOW_BYTE;
      }
      break;
    case parser_state_t::WAIT_CRC_LOW_BYTE:
      parser.crc_low_byte = byte;
      parser.state = parser_state_t::WAIT_CRC_HIGH_BYTE;
      break;
    case parser_state_t::WAIT_CRC_HIGH_BYTE:
      parser.crc_high_byte = byte;
      if (parser.version == MAVLINK_VERSION_V2 &&
          (parser.incompatibility_flags & kMavlinkIflagSigned) != 0) {
        signature_bytes_remaining = kMavlinkSignatureLength;
        reset_parser_context(parser, dialect_);
        break;
      }
      if (mavlink_parser_check_crc(parser)) {
        frame_t frame{};
        frame.version = parser.version;
        frame.sequence = parser.sequence;
        frame.system_id = parser.system_id;
        frame.component_id = parser.component_id;
        frame.message_id = parser.message_id;
        frame.payload_len = parser.payload_length;
        const int crc_extra = dialect_->crc_extra(dialect_, parser.message_id);
        frame.crc_extra = crc_extra >= 0 ? static_cast<uint8_t>(crc_extra) : 0;
        mavlink_memcpy_s(frame.payload, sizeof(frame.payload), parser.payload, parser.payload_length);
        emit_frame(frame);
      }
      reset_parser_context(parser, dialect_);
      break;
    default:
      reset_parser_context(parser, dialect_);
      break;
  }
}

void MavlinkSession::emit_frame(const frame_t& frame) {
  std::vector<std::pair<size_t, frame_listener_t>> listeners;
  std::vector<std::shared_ptr<MessageListener>> message_listeners_copy;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) {
      return;
    }

    recent_frames_.push_back(frame);
    if (recent_frames_.size() > kRecentFrameCapacity) {
      recent_frames_.erase(recent_frames_.begin());
    }

    complete_pending_waits(frame);
    listeners = frame_listeners_;
    message_listeners_copy = message_listeners_;
  }

  for (const auto& entry : listeners) {
    if (entry.second) {
      entry.second(frame);
    }
  }

  for (const auto& listener : message_listeners_copy) {
    if (!listener->active || !listener->on_data) {
      continue;
    }
    if (listener->message_id != frame.message_id) {
      continue;
    }
    if (!frame_matches_origin(frame, listener->from_system_id, listener->from_component_id)) {
      continue;
    }
    listener->on_data(frame.payload, frame.payload_len, frame);
  }
}

void MavlinkSession::complete_pending_waits(const frame_t& frame) {
  for (auto& pending : pending_waits_) {
    if (pending->completed) {
      continue;
    }
    if (!pending->predicate(frame)) {
      continue;
    }
    pending->completed = true;
    pending->success = true;
    pending->result = frame;
    pending->cv.notify_all();
    break;
  }
}

bool MavlinkSession::check_recent_frames(frame_predicate_t predicate, frame_t& out_frame) {
  for (auto it = recent_frames_.begin(); it != recent_frames_.end(); ++it) {
    if (!predicate(*it)) {
      continue;
    }
    out_frame = *it;
    recent_frames_.erase(it);
    return true;
  }
  return false;
}

}  // namespace mavlink
