#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <vector>

namespace mavlink {

class VirtualMavlinkEndpoint;

/// Transport-agnostic MAVLink byte stream.
///
/// Protocol classes depend only on [MavlinkLink], not on serial/UDP/TCP.
class MavlinkLink {
 public:
  using receive_handler_t = std::function<void(const uint8_t* data, size_t len)>;

  virtual ~MavlinkLink() = default;

  /// Send raw MAVLink frame bytes to the remote peer.
  virtual void send(const uint8_t* data, size_t len) = 0;

  /// Register a handler for incoming raw bytes.
  virtual void set_receive_handler(receive_handler_t handler) = 0;

  /// Release link resources. Default implementation is a no-op.
  virtual void close() {}
};

/// In-memory link for tests and virtual examples.
///
/// Bytes sent by one endpoint are delivered to every other endpoint on the bus.
class VirtualMavlinkBus {
 public:
  VirtualMavlinkBus() = default;
  ~VirtualMavlinkBus();

  VirtualMavlinkBus(const VirtualMavlinkBus&) = delete;
  VirtualMavlinkBus& operator=(const VirtualMavlinkBus&) = delete;

  /// Create a new endpoint on this bus.
  std::shared_ptr<MavlinkLink> create_endpoint();

  /// Close every endpoint on the bus.
  void close_all();

 private:
  friend class VirtualMavlinkEndpoint;

  void deliver(const uint8_t* data, size_t len, VirtualMavlinkEndpoint* sender);

  std::mutex mutex_;
  std::vector<VirtualMavlinkEndpoint*> endpoints_;
};

class VirtualMavlinkEndpoint : public MavlinkLink {
 public:
  explicit VirtualMavlinkEndpoint(VirtualMavlinkBus* bus);
  ~VirtualMavlinkEndpoint() override;

  void send(const uint8_t* data, size_t len) override;
  void set_receive_handler(receive_handler_t handler) override;
  void close() override;

 private:
  friend class VirtualMavlinkBus;

  void emit(const uint8_t* data, size_t len);

  VirtualMavlinkBus* bus_;
  receive_handler_t receive_handler_;
  bool closed_ = false;
  std::mutex mutex_;
};

}  // namespace mavlink
