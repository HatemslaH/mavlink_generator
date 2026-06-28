#pragma once

#include <string>
#include <vector>

namespace sitl_gcs {

struct SerialPortInfo {
  std::string path;
  std::string description;
};

/// List serial ports available on this host.
std::vector<SerialPortInfo> list_serial_ports();

/// List ports and read a selection from stdin.
std::string pick_serial_port();

/// Parse `--baud <rate>` from CLI arguments (default 57600).
int parse_baud_rate(int argc, char* argv[], int default_baud = 57600);

}  // namespace sitl_gcs
