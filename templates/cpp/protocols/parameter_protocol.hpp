#pragma once

#include <chrono>
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "../mavlink.hpp"
#include "mavlink_cancellation.hpp"
#include "mavlink_session.hpp"
#include "param_codec.hpp"

namespace mavlink {

/// Decoded onboard parameter entry.
struct ParamEntry {
  std::string id;
  double value;
  MAV_PARAM_TYPE type;
  uint16_t index;
  uint16_t count;

  static ParamEntry from_param_value(const param_value_t& message);
};

using ParamProgressCallback = std::function<void(const ParamEntry& entry, int received, int expected)>;

/// GCS-side MAVLink parameter protocol client.
class ParameterProtocol {
 public:
  ParameterProtocol(
    MavlinkSession* session,
    uint8_t target_system,
    uint8_t target_component,
    std::chrono::milliseconds idle_timeout = std::chrono::milliseconds(500),
    std::chrono::milliseconds request_timeout = std::chrono::seconds(3)
  );

  const std::map<std::string, ParamEntry>& cache() const;
  void clear_cache();

  std::optional<MAV_PARAM_TYPE> type_for_name(const std::string& name) const;

  std::vector<ParamEntry> fetch_all(
    ParamProgressCallback on_progress = nullptr,
    MavlinkCancellationToken* cancel = nullptr
  );

  ParamEntry read_by_name(const std::string& name, MavlinkCancellationToken* cancel = nullptr);
  ParamEntry read_by_index(int index, MavlinkCancellationToken* cancel = nullptr);
  ParamEntry read(
    const char* param_id = nullptr,
    int param_index = -1,
    MavlinkCancellationToken* cancel = nullptr
  );

  ParamEntry write(
    const std::string& name,
    double value,
    MAV_PARAM_TYPE type,
    MavlinkCancellationToken* cancel = nullptr
  );

  ParamEntry write_by_name(
    const std::string& name,
    double value,
    std::optional<MAV_PARAM_TYPE> type = std::nullopt,
    MavlinkCancellationToken* cancel = nullptr
  );

 private:
  void remember(const ParamEntry& entry);

  MavlinkSession* session_;
  uint8_t target_system_;
  uint8_t target_component_;
  std::chrono::milliseconds idle_timeout_;
  std::chrono::milliseconds request_timeout_;
  std::map<std::string, ParamEntry> cache_;
};

struct ParamStoredValue {
  double value;
  MAV_PARAM_TYPE type;
};

/// Vehicle-side parameter store handler.
class ParameterServer {
 public:
  ParameterServer(
    MavlinkSession* session,
    const std::map<std::string, ParamStoredValue>* initial_values = nullptr
  );

  ~ParameterServer();

  const std::map<std::string, ParamStoredValue>& values() const;
  void set(const std::string& name, double value, MAV_PARAM_TYPE type);
  void close();

 private:
  void on_frame(const frame_t& frame);
  void broadcast_all();
  void send_value(const std::string& name, const ParamStoredValue& entry, int index);
  std::optional<std::pair<std::string, ParamStoredValue>> resolve_read(
    const param_request_read_t& request
  ) const;
  int index_of(const std::string& name) const;

  MavlinkSession* session_;
  std::map<std::string, ParamStoredValue> values_;
  size_t frame_listener_id_ = 0;
  bool closed_ = false;
};

}  // namespace mavlink
