#include "port_picker.hpp"

#include <cstdio>
#include <cstdlib>
#include <stdexcept>
#include <string>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#else
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#endif

namespace sitl_gcs {

namespace {

#ifdef _WIN32
void append_registry_ports(std::vector<SerialPortInfo>& ports) {
  HKEY key = nullptr;
  if (RegOpenKeyExA(
        HKEY_LOCAL_MACHINE,
        R"(HARDWARE\DEVICEMAP\SERIALCOMM)",
        0,
        KEY_READ,
        &key
      ) != ERROR_SUCCESS) {
    return;
  }

  DWORD index = 0;
  char value_name[256];
  char port_name[256];
  DWORD value_name_len = 0;
  DWORD port_name_len = 0;
  DWORD type = 0;

  while (true) {
    value_name_len = sizeof(value_name);
    port_name_len = sizeof(port_name);
    const LONG result = RegEnumValueA(
      key,
      index++,
      value_name,
      &value_name_len,
      nullptr,
      &type,
      reinterpret_cast<LPBYTE>(port_name),
      &port_name_len
    );
    if (result == ERROR_NO_MORE_ITEMS) {
      break;
    }
    if (result != ERROR_SUCCESS || type != REG_SZ) {
      continue;
    }
    ports.push_back(SerialPortInfo{port_name, value_name});
  }

  RegCloseKey(key);
}
#else
bool is_char_device(const std::string& path) {
  struct stat st{};
  if (stat(path.c_str(), &st) != 0) {
    return false;
  }
  return S_ISCHR(st.st_mode);
}

void append_if_exists(std::vector<SerialPortInfo>& ports, const std::string& path, const char* description) {
  if (!is_char_device(path)) {
    return;
  }
  for (const auto& existing : ports) {
    if (existing.path == path) {
      return;
    }
  }
  ports.push_back(SerialPortInfo{path, description});
}

void append_glob_like(std::vector<SerialPortInfo>& ports, const std::string& directory, const char* prefix) {
  DIR* dir = opendir(directory.c_str());
  if (dir == nullptr) {
    return;
  }

  while (const dirent* entry = readdir(dir)) {
    const std::string name = entry->d_name;
    if (name == "." || name == "..") {
      continue;
    }
    if (name.rfind(prefix, 0) != 0) {
      continue;
    }
    append_if_exists(ports, directory + "/" + name, prefix);
  }

  closedir(dir);
}
#endif

}  // namespace

std::vector<SerialPortInfo> list_serial_ports() {
  std::vector<SerialPortInfo> ports;

#ifdef _WIN32
  append_registry_ports(ports);
#else
#if defined(__APPLE__)
  append_glob_like(ports, "/dev", "cu.");
  append_glob_like(ports, "/dev", "tty.usb");
#else
  append_glob_like(ports, "/dev", "ttyUSB");
  append_glob_like(ports, "/dev", "ttyACM");
#endif
  append_if_exists(ports, "/dev/ttyS0", "ttyS0");
  append_if_exists(ports, "/dev/ttyS1", "ttyS1");
#endif

  return ports;
}

std::string pick_serial_port() {
  const auto ports = list_serial_ports();
  if (ports.empty()) {
    throw std::runtime_error("No serial ports found. Connect SITL or a USB adapter.");
  }

  std::printf("\nAvailable serial ports:\n");
  for (size_t index = 0; index < ports.size(); ++index) {
    const auto& info = ports[index];
    if (info.description.empty()) {
      std::printf("  [%zu] %s\n", index, info.path.c_str());
    } else {
      std::printf("  [%zu] %s (%s)\n", index, info.path.c_str(), info.description.c_str());
    }
  }

  std::printf("\nSelect port [0-%zu]: ", ports.size() - 1);
  char line[128];
  if (std::fgets(line, sizeof(line), stdin) == nullptr) {
    throw std::runtime_error("Port selection required");
  }

  char* end = nullptr;
  const long selected = std::strtol(line, &end, 10);
  if (end == line || selected < 0 || static_cast<size_t>(selected) >= ports.size()) {
    throw std::runtime_error(std::string("Invalid port selection: ") + line);
  }

  const std::string port_name = ports[static_cast<size_t>(selected)].path;
  std::printf("Selected %s\n", port_name.c_str());
  return port_name;
}

int parse_baud_rate(int argc, char* argv[], int default_baud) {
  for (int index = 1; index < argc - 1; ++index) {
    if (std::string(argv[index]) == "--baud") {
      char* end = nullptr;
      const long value = std::strtol(argv[index + 1], &end, 10);
      if (end == argv[index + 1] || value <= 0) {
        throw std::invalid_argument(std::string("Invalid --baud value: ") + argv[index + 1]);
      }
      return static_cast<int>(value);
    }
  }
  return default_baud;
}

}  // namespace sitl_gcs
