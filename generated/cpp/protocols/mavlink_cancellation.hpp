#pragma once

#include <exception>
#include <functional>
#include <mutex>
#include <string>
#include <vector>

namespace mavlink {

/// Thrown when a MAVLink wait or long-running protocol operation is cancelled.
class MavlinkCancelledException : public std::exception {
 public:
  explicit MavlinkCancelledException(const char* message = "Operation cancelled")
      : message_(message) {}

  const char* what() const noexcept override { return message_.c_str(); }

 private:
  std::string message_;
};

/// Cooperative cancellation token for [MavlinkSession] waits and protocol flows.
class MavlinkCancellationToken {
 public:
  MavlinkCancellationToken() = default;

  bool is_cancelled() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return cancelled_;
  }

  void cancel() {
    std::vector<std::function<void()>> listeners;
    {
      std::lock_guard<std::mutex> lock(mutex_);
      if (cancelled_) {
        return;
      }
      cancelled_ = true;
      listeners = std::move(on_cancel_);
      on_cancel_.clear();
    }
    for (const auto& listener : listeners) {
      if (listener) {
        listener();
      }
    }
  }

  void throw_if_cancelled() const {
    if (is_cancelled()) {
      throw MavlinkCancelledException();
    }
  }

  /// Register a one-shot listener invoked when [cancel] is called.
  void on_cancel(std::function<void()> listener) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (cancelled_) {
      if (listener) {
        listener();
      }
      return;
    }
    on_cancel_.push_back(std::move(listener));
  }

  void dispose() {
    std::lock_guard<std::mutex> lock(mutex_);
    on_cancel_.clear();
  }

 private:
  mutable std::mutex mutex_;
  bool cancelled_ = false;
  std::vector<std::function<void()>> on_cancel_;
};

}  // namespace mavlink
