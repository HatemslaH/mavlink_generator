#include "heartbeat_protocol.hpp"

#include <condition_variable>
#include <cstring>

namespace mavlink {

HeartbeatMonitor::HeartbeatMonitor(
  MavlinkSession* session,
  std::chrono::milliseconds timeout,
  std::optional<std::set<MavlinkNode>> watch,
  std::optional<uint8_t> watch_system_id
)
    : session_(session),
      timeout_(timeout),
      watch_(std::move(watch)),
      watch_system_id_(watch_system_id) {}

HeartbeatMonitor::~HeartbeatMonitor() { stop(); }

void HeartbeatMonitor::start() {
  std::lock_guard<std::mutex> lock(mutex_);
  if (running_) {
    return;
  }
  running_ = true;
  frame_listener_id_ = session_->add_frame_listener([this](const frame_t& frame) { on_frame(frame); });

  watchdog_thread_ = std::make_unique<std::thread>([this]() {
    while (running_) {
      std::this_thread::sleep_for(timeout_ / 3);
      if (!running_) {
        break;
      }
      check_timeouts();
    }
  });
}

void HeartbeatMonitor::stop() {
  {
    std::lock_guard<std::mutex> lock(mutex_);
    if (!running_) {
      return;
    }
    running_ = false;
  }

  if (watchdog_thread_ && watchdog_thread_->joinable()) {
    watchdog_thread_->join();
  }
  watchdog_thread_.reset();
  session_->remove_frame_listener(frame_listener_id_);
}

void HeartbeatMonitor::on_heartbeat(heartbeat_callback_t callback) {
  std::lock_guard<std::mutex> lock(mutex_);
  heartbeat_callbacks_.push_back(std::move(callback));
}

void HeartbeatMonitor::on_connected(node_callback_t callback) {
  std::lock_guard<std::mutex> lock(mutex_);
  connected_callbacks_.push_back(std::move(callback));
}

void HeartbeatMonitor::on_disconnected(node_callback_t callback) {
  std::lock_guard<std::mutex> lock(mutex_);
  disconnected_callbacks_.push_back(std::move(callback));
}

std::optional<TrackedHeartbeat> HeartbeatMonitor::state_for(const MavlinkNode& node) const {
  std::lock_guard<std::mutex> lock(mutex_);
  const auto it = states_.find(node);
  if (it == states_.end()) {
    return std::nullopt;
  }
  return it->second;
}

std::optional<TrackedHeartbeat> HeartbeatMonitor::state_for_ids(
  uint8_t system_id,
  uint8_t component_id
) const {
  return state_for(MavlinkNode{system_id, component_id});
}

bool HeartbeatMonitor::is_online(const MavlinkNode& node) const {
  std::lock_guard<std::mutex> lock(mutex_);
  const auto it = online_.find(node);
  return it != online_.end() && it->second;
}

bool HeartbeatMonitor::is_online_ids(uint8_t system_id, uint8_t component_id) const {
  return is_online(MavlinkNode{system_id, component_id});
}

std::vector<MavlinkNode> HeartbeatMonitor::online_nodes() const {
  std::lock_guard<std::mutex> lock(mutex_);
  std::vector<MavlinkNode> nodes;
  for (const auto& entry : online_) {
    if (entry.second) {
      nodes.push_back(entry.first);
    }
  }
  return nodes;
}

MavlinkNode HeartbeatMonitor::wait_for_vehicle(
  const std::set<uint8_t>* exclude_system_ids,
  std::chrono::milliseconds timeout,
  MavlinkCancellationToken* cancel
) {
  if (cancel != nullptr) {
    cancel->throw_if_cancelled();
  }

  for (const auto& node : online_nodes()) {
    if (exclude_system_ids == nullptr || exclude_system_ids->count(node.system_id) == 0) {
      return node;
    }
  }

  std::mutex wait_mutex;
  std::condition_variable cv;
  bool done = false;
  MavlinkNode result{0, 0};

  auto connected_cb = [&](const MavlinkNode& node) {
    if (exclude_system_ids != nullptr && exclude_system_ids->count(node.system_id) != 0) {
      return;
    }
    std::lock_guard<std::mutex> lock(wait_mutex);
    if (!done) {
      done = true;
      result = node;
      cv.notify_all();
    }
  };

  on_connected(connected_cb);

  std::unique_lock<std::mutex> lock(wait_mutex);
  if (cancel != nullptr) {
    cancel->on_cancel([&]() {
      std::lock_guard<std::mutex> inner(wait_mutex);
      done = true;
      cv.notify_all();
    });
  }

  if (!cv.wait_for(lock, timeout, [&]() { return done; })) {
    throw MavlinkTimeoutException("Timed out waiting for vehicle heartbeat", timeout);
  }

  if (cancel != nullptr && cancel->is_cancelled()) {
    throw MavlinkCancelledException();
  }

  return result;
}

void HeartbeatMonitor::on_frame(const frame_t& frame) {
  if (frame.message_id != heartbeat_MSG_ID) {
    return;
  }

  heartbeat_t heartbeat_msg{};
  heartbeat_parse(frame.payload, heartbeat_msg);

  const MavlinkNode node{frame.system_id, frame.component_id};
  if (!should_watch(node)) {
    return;
  }

  std::vector<heartbeat_callback_t> heartbeat_cbs;
  std::vector<node_callback_t> connected_cbs;
  TrackedHeartbeat tracked{};

  {
    std::lock_guard<std::mutex> lock(mutex_);
    const bool was_online = online_.count(node) != 0 && online_[node];
    tracked = TrackedHeartbeat{
      node,
      heartbeat_msg,
      std::chrono::steady_clock::now(),
      true,
    };
    states_[node] = tracked;
    online_[node] = true;
    heartbeat_cbs = heartbeat_callbacks_;

    if (!was_online) {
      connected_cbs = connected_callbacks_;
    }
  }

  for (const auto& cb : heartbeat_cbs) {
    if (cb) {
      cb(tracked);
    }
  }
  for (const auto& cb : connected_cbs) {
    if (cb) {
      cb(node);
    }
  }
}

void HeartbeatMonitor::check_timeouts() {
  const auto now = std::chrono::steady_clock::now();
  std::vector<node_callback_t> disconnected_cbs;
  std::vector<heartbeat_callback_t> heartbeat_cbs;
  std::vector<TrackedHeartbeat> offline_updates;

  {
    std::lock_guard<std::mutex> lock(mutex_);
    for (const auto& entry : states_) {
      const auto& node = entry.first;
      const auto& state = entry.second;
      const bool timed_out = now - state.received_at > timeout_;
      const bool was_online = online_.count(node) != 0 && online_[node];

      if (timed_out && was_online) {
        online_[node] = false;
        offline_updates.push_back(TrackedHeartbeat{
          node,
          state.heartbeat_msg,
          state.received_at,
          false,
        });
        disconnected_cbs = disconnected_callbacks_;
        heartbeat_cbs = heartbeat_callbacks_;
      }
    }
  }

  for (const auto& tracked : offline_updates) {
    for (const auto& cb : heartbeat_cbs) {
      if (cb) {
        cb(tracked);
      }
    }
    for (const auto& cb : disconnected_cbs) {
      if (cb) {
        cb(tracked.node);
      }
    }
  }
}

bool HeartbeatMonitor::should_watch(const MavlinkNode& node) const {
  if (watch_.has_value()) {
    return watch_->count(node) != 0;
  }
  if (watch_system_id_.has_value()) {
    return node.system_id == watch_system_id_.value();
  }
  return true;
}

HeartbeatPublisher::HeartbeatPublisher(
  MavlinkSession* session,
  const heartbeat_t& heartbeat_msg,
  std::chrono::milliseconds interval
)
    : session_(session), heartbeat_msg_(heartbeat_msg), interval_(interval) {}

HeartbeatPublisher::~HeartbeatPublisher() { stop(); }

const heartbeat_t& HeartbeatPublisher::heartbeat_msg() const { return heartbeat_msg_; }

void HeartbeatPublisher::update_heartbeat(const heartbeat_t& heartbeat_msg) {
  heartbeat_msg_ = heartbeat_msg;
}

void HeartbeatPublisher::mutate_heartbeat(const std::function<void(heartbeat_t&)>& transform) {
  transform(heartbeat_msg_);
}

void HeartbeatPublisher::start() {
  if (running_.exchange(true)) {
    return;
  }
  send_once();
  thread_ = std::make_unique<std::thread>([this]() { publish_loop(); });
}

void HeartbeatPublisher::stop() {
  if (!running_.exchange(false)) {
    return;
  }
  if (thread_ && thread_->joinable()) {
    thread_->join();
  }
  thread_.reset();
}

void HeartbeatPublisher::send_once() {
  uint8_t payload[heartbeat_ENCODED_LENGTH];
  heartbeat_serialize(heartbeat_msg_, payload);
  session_->send_frame(
    heartbeat_MSG_ID,
    heartbeat_CRC_EXTRA,
    payload,
    heartbeat_ENCODED_LENGTH
  );
}

void HeartbeatPublisher::publish_loop() {
  while (running_) {
    std::this_thread::sleep_for(interval_);
    if (!running_) {
      break;
    }
    send_once();
  }
}

heartbeat_t HeartbeatTemplates::gcs(int mavlink_version) {
  heartbeat_t hb{};
  hb.custom_mode = 0;
  hb.type = MAV_TYPE_GCS;
  hb.autopilot = MAV_AUTOPILOT_INVALID;
  hb.base_mode = 0;
  hb.system_status = MAV_STATE_ACTIVE;
  hb.mavlink_version = static_cast<uint8_t>(mavlink_version);
  return hb;
}

heartbeat_t HeartbeatTemplates::autopilot(
  int mavlink_version,
  MAV_TYPE type,
  MAV_AUTOPILOT autopilot,
  MAV_STATE system_status,
  uint32_t custom_mode,
  uint8_t base_mode
) {
  heartbeat_t hb{};
  hb.custom_mode = custom_mode;
  hb.type = type;
  hb.autopilot = autopilot;
  hb.base_mode = base_mode;
  hb.system_status = system_status;
  hb.mavlink_version = static_cast<uint8_t>(mavlink_version);
  return hb;
}

heartbeat_t HeartbeatTemplates::onboard_api(int mavlink_version) {
  heartbeat_t hb{};
  hb.custom_mode = 0;
  hb.type = MAV_TYPE_ONBOARD_CONTROLLER;
  hb.autopilot = MAV_AUTOPILOT_INVALID;
  hb.base_mode = 0;
  hb.system_status = MAV_STATE_ACTIVE;
  hb.mavlink_version = static_cast<uint8_t>(mavlink_version);
  return hb;
}

}  // namespace mavlink
