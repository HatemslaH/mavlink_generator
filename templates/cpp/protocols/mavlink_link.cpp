#include "mavlink_link.hpp"

#include <algorithm>
#include <memory>
#include <stdexcept>

namespace mavlink {

VirtualMavlinkBus::~VirtualMavlinkBus() { close_all(); }

std::shared_ptr<MavlinkLink> VirtualMavlinkBus::create_endpoint() {
  auto endpoint = std::shared_ptr<VirtualMavlinkEndpoint>(new VirtualMavlinkEndpoint(this));
  std::lock_guard<std::mutex> lock(mutex_);
  endpoints_.push_back(endpoint.get());
  return endpoint;
}

void VirtualMavlinkBus::close_all() {
  std::vector<VirtualMavlinkEndpoint*> copy;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    copy = endpoints_;
    endpoints_.clear();
  }
  for (auto* endpoint : copy) {
    endpoint->close();
  }
}

void VirtualMavlinkBus::deliver(
  const uint8_t* data,
  size_t len,
  VirtualMavlinkEndpoint* sender
) {
  std::vector<VirtualMavlinkEndpoint*> copy;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    copy = endpoints_;
  }
  for (auto* endpoint : copy) {
    if (endpoint != sender) {
      endpoint->emit(data, len);
    }
  }
}

VirtualMavlinkEndpoint::VirtualMavlinkEndpoint(VirtualMavlinkBus* bus) : bus_(bus) {}

VirtualMavlinkEndpoint::~VirtualMavlinkEndpoint() { close(); }

void VirtualMavlinkEndpoint::send(const uint8_t* data, size_t len) {
  std::lock_guard<std::mutex> lock(mutex_);
  if (closed_) {
    throw std::runtime_error("VirtualMavlinkEndpoint is closed");
  }
  bus_->deliver(data, len, this);
}

void VirtualMavlinkEndpoint::set_receive_handler(receive_handler_t handler) {
  std::lock_guard<std::mutex> lock(mutex_);
  receive_handler_ = std::move(handler);
}

void VirtualMavlinkEndpoint::close() {
  std::lock_guard<std::mutex> lock(mutex_);
  if (closed_) {
    return;
  }
  closed_ = true;
  receive_handler_ = nullptr;
  std::lock_guard<std::mutex> bus_lock(bus_->mutex_);
  auto& endpoints = bus_->endpoints_;
  endpoints.erase(
    std::remove(endpoints.begin(), endpoints.end(), this),
    endpoints.end()
  );
}

void VirtualMavlinkEndpoint::emit(const uint8_t* data, size_t len) {
  receive_handler_t handler;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) {
      return;
    }
    handler = receive_handler_;
  }
  if (handler) {
    handler(data, len);
  }
}

}  // namespace mavlink
