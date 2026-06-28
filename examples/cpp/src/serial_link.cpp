#include "serial_link.hpp"

#include <chrono>
#include <stdexcept>
#include <vector>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#else
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <termios.h>
#include <unistd.h>
#endif

namespace sitl_gcs {

namespace {

#ifdef _WIN32
std::string win32_error_message(DWORD error) {
  char* buffer = nullptr;
  const DWORD flags = FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM |
                      FORMAT_MESSAGE_IGNORE_INSERTS;
  const DWORD length = FormatMessageA(
    flags,
    nullptr,
    error,
    MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
    reinterpret_cast<LPSTR>(&buffer),
    0,
    nullptr
  );
  std::string message = length > 0 && buffer != nullptr ? std::string(buffer, length) : "unknown error";
  if (buffer != nullptr) {
    LocalFree(buffer);
  }
  return message;
}

void configure_port(HANDLE handle, int baud_rate) {
  DCB dcb{};
  dcb.DCBlength = sizeof(DCB);
  if (!GetCommState(handle, &dcb)) {
    throw std::runtime_error("GetCommState failed: " + win32_error_message(GetLastError()));
  }

  dcb.BaudRate = static_cast<DWORD>(baud_rate);
  dcb.ByteSize = 8;
  dcb.Parity = NOPARITY;
  dcb.StopBits = ONESTOPBIT;
  dcb.fBinary = TRUE;
  dcb.fDtrControl = DTR_CONTROL_ENABLE;
  dcb.fRtsControl = RTS_CONTROL_ENABLE;

  if (!SetCommState(handle, &dcb)) {
    throw std::runtime_error("SetCommState failed: " + win32_error_message(GetLastError()));
  }

  COMMTIMEOUTS timeouts{};
  timeouts.ReadIntervalTimeout = MAXDWORD;
  timeouts.ReadTotalTimeoutConstant = 50;
  timeouts.ReadTotalTimeoutMultiplier = 0;
  timeouts.WriteTotalTimeoutConstant = 1000;
  timeouts.WriteTotalTimeoutMultiplier = 0;
  if (!SetCommTimeouts(handle, &timeouts)) {
    throw std::runtime_error("SetCommTimeouts failed: " + win32_error_message(GetLastError()));
  }
}

std::string normalize_port_name(const std::string& port_name) {
  if (port_name.rfind(R"(\\.\)", 0) == 0) {
    return port_name;
  }
  return R"(\\.\)" + port_name;
}
#else
speed_t baud_to_speed(int baud_rate) {
  switch (baud_rate) {
    case 9600:
      return B9600;
    case 19200:
      return B19200;
    case 38400:
      return B38400;
    case 57600:
      return B57600;
    case 115200:
      return B115200;
    case 230400:
      return B230400;
    case 460800:
      return B460800;
    case 921600:
      return B921600;
    default:
      throw std::invalid_argument("Unsupported baud rate on this platform: " + std::to_string(baud_rate));
  }
}

void configure_port(int fd, int baud_rate) {
  termios options{};
  if (tcgetattr(fd, &options) != 0) {
    throw std::runtime_error(std::string("tcgetattr failed: ") + std::strerror(errno));
  }

  cfmakeraw(&options);
  const speed_t speed = baud_to_speed(baud_rate);
  cfsetispeed(&options, speed);
  cfsetospeed(&options, speed);
  options.c_cflag |= (CLOCAL | CREAD);
  options.c_cflag &= ~CSIZE;
  options.c_cflag |= CS8;
  options.c_cflag &= ~PARENB;
  options.c_cflag &= ~CSTOPB;
  options.c_cc[VMIN] = 0;
  options.c_cc[VTIME] = 1;

  if (tcsetattr(fd, TCSANOW, &options) != 0) {
    throw std::runtime_error(std::string("tcsetattr failed: ") + std::strerror(errno));
  }
}
#endif

}  // namespace

SerialMavlinkLink::SerialMavlinkLink(const std::string& port_name, int baud_rate)
    : port_name_(port_name), baud_rate_(baud_rate) {}

SerialMavlinkLink::~SerialMavlinkLink() { close(); }

std::shared_ptr<SerialMavlinkLink> SerialMavlinkLink::open(
  const std::string& port_name,
  int baud_rate
) {
  auto link = std::shared_ptr<SerialMavlinkLink>(new SerialMavlinkLink(port_name, baud_rate));

#ifdef _WIN32
  const std::string device_path = normalize_port_name(port_name);
  HANDLE handle = CreateFileA(
    device_path.c_str(),
    GENERIC_READ | GENERIC_WRITE,
    0,
    nullptr,
    OPEN_EXISTING,
    FILE_ATTRIBUTE_NORMAL,
    nullptr
  );
  if (handle == INVALID_HANDLE_VALUE) {
    throw std::runtime_error(
      "Failed to open " + port_name + ": " + win32_error_message(GetLastError())
    );
  }
  configure_port(handle, baud_rate);
  link->handle_ = handle;
#else
  const int fd = ::open(port_name.c_str(), O_RDWR | O_NOCTTY | O_NONBLOCK);
  if (fd < 0) {
    throw std::runtime_error(
      std::string("Failed to open ") + port_name + ": " + std::strerror(errno)
    );
  }
  configure_port(fd, baud_rate);
  link->fd_ = fd;
#endif

  link->read_loop_running_ = true;
  link->reader_thread_ = std::thread(&SerialMavlinkLink::read_loop, link.get());
  return link;
}

void SerialMavlinkLink::send(const uint8_t* data, size_t len) {
  if (closed_.load()) {
    throw std::runtime_error("SerialMavlinkLink is closed");
  }
  if (data == nullptr || len == 0) {
    return;
  }

#ifdef _WIN32
  HANDLE handle = static_cast<HANDLE>(handle_);
  DWORD written = 0;
  if (!WriteFile(handle, data, static_cast<DWORD>(len), &written, nullptr) || written != len) {
    throw std::runtime_error("Serial write failed on " + port_name_);
  }
#else
  size_t offset = 0;
  while (offset < len) {
    const ssize_t written = ::write(fd_, data + offset, len - offset);
    if (written < 0) {
      if (errno == EINTR) {
        continue;
      }
      throw std::runtime_error(
        std::string("Serial write failed on ") + port_name_ + ": " + std::strerror(errno)
      );
    }
    if (written == 0) {
      throw std::runtime_error("Serial write returned 0 on " + port_name_);
    }
    offset += static_cast<size_t>(written);
  }
#endif
}

void SerialMavlinkLink::set_receive_handler(receive_handler_t handler) {
  std::lock_guard<std::mutex> lock(handler_mutex_);
  receive_handler_ = std::move(handler);
}

void SerialMavlinkLink::close() {
  if (closed_.exchange(true)) {
    return;
  }

#ifdef _WIN32
  if (handle_ != reinterpret_cast<void*>(static_cast<intptr_t>(-1))) {
    CloseHandle(static_cast<HANDLE>(handle_));
    handle_ = reinterpret_cast<void*>(static_cast<intptr_t>(-1));
  }
#else
  if (fd_ >= 0) {
    ::close(fd_);
    fd_ = -1;
  }
#endif

  if (read_loop_running_.load() && reader_thread_.joinable()) {
    reader_thread_.join();
  }
  read_loop_running_ = false;
}

void SerialMavlinkLink::read_loop() {
  std::vector<uint8_t> buffer(4096);

  while (!closed_.load()) {
#ifdef _WIN32
    HANDLE handle = static_cast<HANDLE>(handle_);
    if (handle == INVALID_HANDLE_VALUE) {
      break;
    }
    DWORD bytes_read = 0;
    if (!ReadFile(handle, buffer.data(), static_cast<DWORD>(buffer.size()), &bytes_read, nullptr)) {
      break;
    }
    if (bytes_read == 0) {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
      continue;
    }
    const size_t len = static_cast<size_t>(bytes_read);
#else
    if (fd_ < 0) {
      break;
    }
    const ssize_t bytes_read = ::read(fd_, buffer.data(), buffer.size());
    if (bytes_read < 0) {
      if (errno == EAGAIN || errno == EWOULDBLOCK) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        continue;
      }
      break;
    }
    if (bytes_read == 0) {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
      continue;
    }
    const size_t len = static_cast<size_t>(bytes_read);
#endif

    receive_handler_t handler;
    {
      std::lock_guard<std::mutex> lock(handler_mutex_);
      handler = receive_handler_;
    }
    if (handler) {
      handler(buffer.data(), len);
    }
  }

  read_loop_running_ = false;
}

}  // namespace sitl_gcs
