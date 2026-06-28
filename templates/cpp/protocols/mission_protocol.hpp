#pragma once

#include <chrono>
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <vector>

#include "../mavlink.hpp"
#include "command_protocol.hpp"
#include "mavlink_cancellation.hpp"
#include "mavlink_session.hpp"

namespace mavlink {

/// Helpers for building and converting mission plan items.
class MissionItems {
 public:
  MissionItems() = delete;

  static mission_item_int_t waypoint(
    uint16_t seq,
    double latitude,
    double longitude,
    float altitude,
    uint8_t target_system,
    uint8_t target_component,
    MAV_CMD command = MAV_CMD_NAV_WAYPOINT,
    MAV_FRAME frame = MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
    MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION,
    float param1 = 0,
    float param2 = 0,
    float param3 = 0,
    float param4 = 0,
    uint8_t current = 0,
    uint8_t autocontinue = 1
  );

  static mission_item_t to_legacy_item(const mission_item_int_t& item);
  static mission_item_int_t from_legacy_item(const mission_item_t& item);
  static std::vector<mission_item_int_t> with_sequential_seq(
    const std::vector<mission_item_int_t>& items
  );
};

using MissionUploadProgressCallback =
  std::function<void(int sent, int total, const mission_item_int_t& item)>;
using MissionDownloadProgressCallback =
  std::function<void(int received, int total, const mission_item_int_t& item)>;

struct MissionSetCurrentResult {
  uint16_t sequence;
  std::optional<command_ack_t> command_ack;
};

/// GCS-side MAVLink mission protocol client.
class MissionProtocol {
 public:
  MissionProtocol(
    MavlinkSession* session,
    uint8_t target_system,
    uint8_t target_component,
    std::chrono::milliseconds item_timeout = std::chrono::seconds(3),
    std::chrono::milliseconds operation_timeout = std::chrono::seconds(10)
  );

  MAV_MISSION_RESULT upload(
    const std::vector<mission_item_int_t>& items,
    MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION,
    MissionUploadProgressCallback on_progress = nullptr,
    MavlinkCancellationToken* cancel = nullptr
  );

  std::vector<mission_item_int_t> download(
    MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION,
    MissionDownloadProgressCallback on_progress = nullptr,
    MavlinkCancellationToken* cancel = nullptr
  );

  MAV_MISSION_RESULT clear(
    MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION,
    MavlinkCancellationToken* cancel = nullptr
  );

  void set_current(uint16_t seq, MavlinkCancellationToken* cancel = nullptr);

  MissionSetCurrentResult set_current_with_command(
    uint16_t seq,
    CommandProtocol* command = nullptr,
    bool also_send_command = true,
    bool reset_mission = false,
    MavlinkCancellationToken* cancel = nullptr
  );

 private:
  bool is_item_request(
    uint32_t message_id,
    const uint8_t* payload,
    uint16_t seq,
    MAV_MISSION_TYPE mission_type
  ) const;

  MavlinkSession* session_;
  uint8_t target_system_;
  uint8_t target_component_;
  std::chrono::milliseconds item_timeout_;
  std::chrono::milliseconds operation_timeout_;
};

/// Vehicle-side mission protocol handler.
class MissionServer {
 public:
  MissionServer(
    MavlinkSession* session,
    const std::vector<mission_item_int_t>* initial_mission = nullptr,
    MAV_MISSION_TYPE mission_type = MAV_MISSION_TYPE_MISSION
  );

  ~MissionServer();

  const std::vector<mission_item_int_t>& items() const;
  void replace_mission(const std::vector<mission_item_int_t>& items);
  void close();

 private:
  void on_frame(const frame_t& frame);
  bool targets_us(uint32_t message_id, const uint8_t* payload) const;
  bool matches_target(uint8_t target_system, uint8_t target_component) const;

  void store_incoming_item(const frame_t& request_frame, const mission_item_int_t& item);
  void request_upload_item(const frame_t& request_frame, uint16_t seq);
  void send_upload_ack(const frame_t& request_frame);
  void send_requested_item(const frame_t& request_frame, uint16_t seq);

  MavlinkSession* session_;
  MAV_MISSION_TYPE mission_type_;
  std::vector<mission_item_int_t> items_;
  std::map<uint16_t, mission_item_int_t> incoming_;
  std::optional<uint16_t> incoming_count_;
  size_t frame_listener_id_ = 0;
  bool closed_ = false;
};

}  // namespace mavlink
