#pragma once

#include <atomic>
#include <cstdint>
#include <memory>
#include <mutex>
#include <thread>
#include <vector>

#include "protocols/mavlink_link.hpp"

namespace sitl_gcs {

/// [mavlink::MavlinkLink] implementation over a host serial port (cross-platform).
class SerialMavlinkLink : public mavlink::MavlinkLink {
 public:
  ~SerialMavlinkLink() override;

  SerialMavlinkLink(const SerialMavlinkLink&) = delete;
  SerialMavlinkLink& operator=(const SerialMavlinkLink&) = delete;

  /// Open [port_name] at [baud_rate] (MAVLink SITL commonly uses 57600 or 115200).
  static std::shared_ptr<SerialMavlinkLink> open(
    const std::string& port_name,
    int baud_rate = 57600
  );

  void send(const uint8_t* data, size_t len) override;
  void set_receive_handler(receive_handler_t handler) override;
  void close() override;

 private:
  explicit SerialMavlinkLink(const std::string& port_name, int baud_rate);

  void read_loop();

  std::string port_name_;
  int baud_rate_ = 57600;
  receive_handler_t receive_handler_;
  std::mutex handler_mutex_;

  std::atomic<bool> closed_{false};
  std::atomic<bool> read_loop_running_{false};
  std::thread reader_thread_;

#ifdef _WIN32
  void* handle_ = reinterpret_cast<void*>(static_cast<intptr_t>(-1));
#else
  int fd_ = -1;
#endif
};

}  // namespace sitl_gcs
