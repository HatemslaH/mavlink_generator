#pragma once

#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include "../mavlink_dialect.hpp"
#include "../mavlink_frame.hpp"
#include "../mavlink_parser.hpp"
#include "../mavlink_version.hpp"
#include "mavlink_cancellation.hpp"
#include "mavlink_link.hpp"

namespace mavlink {

/// Thrown when an expected MAVLink message is not received in time.
class MavlinkTimeoutException : public std::exception {
 public:
  MavlinkTimeoutException(const char* message, std::chrono::milliseconds timeout)
      : message_(message), timeout_(timeout) {}

  const char* what() const noexcept override { return message_.c_str(); }

  std::chrono::milliseconds timeout() const { return timeout_; }

 private:
  std::string message_;
  std::chrono::milliseconds timeout_;
};

using frame_predicate_t = std::function<bool(const frame_t& frame)>;
using message_predicate_t = std::function<bool(
  uint32_t message_id,
  const uint8_t* payload,
  size_t payload_len,
  uint8_t system_id,
  uint8_t component_id
)>;
using frame_listener_t = std::function<void(const frame_t& frame)>;

/// Handle returned by [MavlinkSession::listen_message]; call [cancel] to unsubscribe.
class MavlinkMessageSubscription {
 public:
  MavlinkMessageSubscription() = default;

  bool is_active() const { return active_; }

  void cancel();

 private:
  friend class MavlinkSession;

  MavlinkMessageSubscription(std::function<void()> cancel_fn, bool active)
      : cancel_fn_(std::move(cancel_fn)), active_(active) {}

  std::function<void()> cancel_fn_;
  bool active_ = false;
};

/// Framing, sequencing, and message dispatch over a [MavlinkLink].
///
/// Protocol implementations use a session to send typed messages and wait for
/// responses without knowing whether the link is USB, UDP, or in-memory.
class MavlinkSession {
 public:
  MavlinkSession(
    const dialect_t* dialect,
    std::shared_ptr<MavlinkLink> link,
    uint8_t system_id,
    uint8_t component_id,
    version_t version = MAVLINK_VERSION_V2
  );
  ~MavlinkSession();

  MavlinkSession(const MavlinkSession&) = delete;
  MavlinkSession& operator=(const MavlinkSession&) = delete;

  const dialect_t* dialect() const { return dialect_; }
  uint8_t system_id() const { return system_id_; }
  uint8_t component_id() const { return component_id_; }
  version_t version() const { return version_; }

  /// Send a serialized MAVLink message as a framed packet.
  void send_frame(uint32_t message_id, uint8_t crc_extra, const uint8_t* payload, size_t payload_len);

  /// Register a listener for all parsed frames. Returns an id for [remove_frame_listener].
  size_t add_frame_listener(frame_listener_t listener);

  void remove_frame_listener(size_t listener_id);

  /// Register a callback for messages matching [message_id]. Returns a subscription handle.
  MavlinkMessageSubscription listen_message(
    uint32_t message_id,
    std::function<void(const uint8_t* payload, size_t payload_len, const frame_t& frame)> on_data,
    std::optional<uint8_t> from_system_id = std::nullopt,
    std::optional<uint8_t> from_component_id = std::nullopt
  );

  /// Wait for the first frame matching [predicate]. Throws on timeout or cancel.
  frame_t wait_for_frame(
    frame_predicate_t predicate,
    std::chrono::milliseconds timeout = std::chrono::seconds(5),
    MavlinkCancellationToken* cancel = nullptr
  );

  /// Wait for the first message matching [predicate]. Throws on timeout or cancel.
  frame_t wait_for_message(
    message_predicate_t predicate,
    std::optional<uint8_t> from_system_id = std::nullopt,
    std::optional<uint8_t> from_component_id = std::nullopt,
    std::chrono::milliseconds timeout = std::chrono::seconds(5),
    MavlinkCancellationToken* cancel = nullptr
  );

  /// Wait for the first message with [message_id]. Throws on timeout or cancel.
  frame_t wait_for_message_id(
    uint32_t message_id,
    std::optional<uint8_t> from_system_id = std::nullopt,
    std::optional<uint8_t> from_component_id = std::nullopt,
    std::chrono::milliseconds timeout = std::chrono::seconds(5),
    MavlinkCancellationToken* cancel = nullptr
  );

  /// Wait for the first message of type [MsgT]. Throws on timeout or cancel.
  template<typename MsgT>
  MsgT wait_for_message_type(
    uint32_t message_id,
    void (*parse_fn)(const uint8_t*, MsgT&),
    std::optional<uint8_t> from_system_id = std::nullopt,
    std::optional<uint8_t> from_component_id = std::nullopt,
    std::chrono::milliseconds timeout = std::chrono::seconds(5),
    MavlinkCancellationToken* cancel = nullptr
  ) {
    const frame_t frame = wait_for_message_id(
      message_id,
      from_system_id,
      from_component_id,
      timeout,
      cancel
    );
    MsgT message{};
    parse_fn(frame.payload, message);
    return message;
  }

  void close();

 private:
  struct PendingWait {
    frame_predicate_t predicate;
    bool completed = false;
    bool success = false;
    frame_t result{};
    std::condition_variable cv;
  };

  struct MessageListener {
    uint32_t message_id;
    std::function<void(const uint8_t*, size_t, const frame_t&)> on_data;
    std::optional<uint8_t> from_system_id;
    std::optional<uint8_t> from_component_id;
    bool active = true;
  };

  void on_link_data(const uint8_t* data, size_t len);
  void feed_byte(uint8_t byte);
  void emit_frame(const frame_t& frame);
  void complete_pending_waits(const frame_t& frame);
  bool check_recent_frames(frame_predicate_t predicate, frame_t& out_frame);

  const dialect_t* dialect_;
  std::shared_ptr<MavlinkLink> link_;
  uint8_t system_id_;
  uint8_t component_id_;
  version_t version_;
  uint8_t sequence_ = 0;

  parser_t parser_{};
  bool closed_ = false;

  std::mutex mutex_;
  std::vector<frame_t> recent_frames_;
  std::vector<std::shared_ptr<PendingWait>> pending_waits_;
  std::vector<std::pair<size_t, frame_listener_t>> frame_listeners_;
  std::vector<std::shared_ptr<MessageListener>> message_listeners_;
  size_t next_listener_id_ = 1;
};

inline void MavlinkMessageSubscription::cancel() {
  if (!active_) {
    return;
  }
  active_ = false;
  if (cancel_fn_) {
    cancel_fn_();
    cancel_fn_ = nullptr;
  }
}

}  // namespace mavlink
