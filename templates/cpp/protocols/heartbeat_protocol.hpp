#pragma once

#include <atomic>
#include <chrono>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <thread>
#include <unordered_map>
#include <vector>

#include "../mavlink.hpp"
#include "mavlink_cancellation.hpp"
#include "mavlink_session.hpp"

namespace mavlink {

/// MAVLink node identity (system + component).
struct MavlinkNode {
  uint8_t system_id;
  uint8_t component_id;

  bool operator==(const MavlinkNode& other) const {
    return system_id == other.system_id && component_id == other.component_id;
  }

  bool operator<(const MavlinkNode& other) const {
    if (system_id != other.system_id) {
      return system_id < other.system_id;
    }
    return component_id < other.component_id;
  }
};

struct MavlinkNodeHash {
  size_t operator()(const MavlinkNode& node) const {
    return (static_cast<size_t>(node.system_id) << 8) | node.component_id;
  }
};

/// Last known heartbeat state for a remote node.
struct TrackedHeartbeat {
  MavlinkNode node;
  heartbeat_t heartbeat_msg;
  std::chrono::steady_clock::time_point received_at;
  bool online;
};

/// Tracks remote HEARTBEAT messages and reports connect / disconnect events.
class HeartbeatMonitor {
 public:
  using heartbeat_callback_t = std::function<void(const TrackedHeartbeat&)>;
  using node_callback_t = std::function<void(const MavlinkNode&)>;

  HeartbeatMonitor(
    MavlinkSession* session,
    std::chrono::milliseconds timeout = std::chrono::seconds(5),
    std::optional<std::set<MavlinkNode>> watch = std::nullopt,
    std::optional<uint8_t> watch_system_id = std::nullopt
  );

  ~HeartbeatMonitor();

  void start();
  void stop();

  void on_heartbeat(heartbeat_callback_t callback);
  void on_connected(node_callback_t callback);
  void on_disconnected(node_callback_t callback);

  std::optional<TrackedHeartbeat> state_for(const MavlinkNode& node) const;
  std::optional<TrackedHeartbeat> state_for_ids(uint8_t system_id, uint8_t component_id) const;

  bool is_online(const MavlinkNode& node) const;
  bool is_online_ids(uint8_t system_id, uint8_t component_id) const;

  std::vector<MavlinkNode> online_nodes() const;

  MavlinkNode wait_for_vehicle(
    const std::set<uint8_t>* exclude_system_ids = nullptr,
    std::chrono::milliseconds timeout = std::chrono::seconds(60),
    MavlinkCancellationToken* cancel = nullptr
  );

 private:
  void on_frame(const frame_t& frame);
  void check_timeouts();
  bool should_watch(const MavlinkNode& node) const;

  MavlinkSession* session_;
  std::chrono::milliseconds timeout_;
  std::optional<std::set<MavlinkNode>> watch_;
  std::optional<uint8_t> watch_system_id_;

  mutable std::mutex mutex_;
  std::unordered_map<MavlinkNode, TrackedHeartbeat, MavlinkNodeHash> states_;
  std::unordered_map<MavlinkNode, bool, MavlinkNodeHash> online_;
  std::vector<heartbeat_callback_t> heartbeat_callbacks_;
  std::vector<node_callback_t> connected_callbacks_;
  std::vector<node_callback_t> disconnected_callbacks_;

  size_t frame_listener_id_ = 0;
  bool running_ = false;
  std::unique_ptr<std::thread> watchdog_thread_;
};

/// Periodically sends HEARTBEAT on a [MavlinkSession].
class HeartbeatPublisher {
 public:
  HeartbeatPublisher(
    MavlinkSession* session,
    const heartbeat_t& heartbeat_msg,
    std::chrono::milliseconds interval = std::chrono::seconds(1)
  );

  ~HeartbeatPublisher();

  const heartbeat_t& heartbeat_msg() const;
  void update_heartbeat(const heartbeat_t& heartbeat_msg);
  void mutate_heartbeat(const std::function<void(heartbeat_t&)>& transform);

  void start();
  void stop();
  void send_once();

 private:
  void publish_loop();

  MavlinkSession* session_;
  heartbeat_t heartbeat_msg_;
  std::chrono::milliseconds interval_;
  std::atomic<bool> running_{false};
  std::unique_ptr<std::thread> thread_;
};

/// Convenience factories for common HEARTBEAT payloads.
class HeartbeatTemplates {
 public:
  HeartbeatTemplates() = delete;

  static heartbeat_t gcs(int mavlink_version);
  static heartbeat_t autopilot(
    int mavlink_version,
    MAV_TYPE type = MAV_TYPE_QUADROTOR,
    MAV_AUTOPILOT autopilot = MAV_AUTOPILOT_PX4,
    MAV_STATE system_status = MAV_STATE_ACTIVE,
    uint32_t custom_mode = 0,
    uint8_t base_mode = 0
  );
  static heartbeat_t onboard_api(int mavlink_version);
};

}  // namespace mavlink
