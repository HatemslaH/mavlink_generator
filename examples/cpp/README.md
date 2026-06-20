# Real C++ GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/cpp` and the transport-agnostic protocol classes.

Synthetic round-trip demos remain under `generated/cpp/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the C++ bindings (at least `rt_rc`):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang cpp
   ```

2. C++17 compiler (GCC 9+, Clang 10+, MSVC 2019+)

3. [CMake](https://cmake.org/) 3.16+

4. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows, `/dev/ttyUSB0` on Linux)

## Build

```bash
cd examples/cpp
cmake -B build
cmake --build build
```

On Windows with Visual Studio:

```powershell
cd examples\cpp
cmake -B build
cmake --build build --config Release
```

The executable is `build/sitl_gcs` (or `build\Release\sitl_gcs.exe` with MSVC multi-config generators).

## Run

```bash
./build/sitl_gcs
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Use `--baud` if your link uses another rate:

```bash
./build/sitl_gcs --baud 115200
```

## Flow

1. **Bootstrap** — `MavlinkGcs::connect` + heartbeats, `wait_for_vehicle`.
2. **Parameters** — `ParameterProtocol::fetch_all` with optional `cancel`.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### CLI commands

| Command | API used |
|---------|----------|
| `cancel` | `MavlinkCancellationToken` on long param/mission ops |
| `att [sec]` | `MavlinkSession::listen_message` + `set_message_interval` |
| `arm` / `disarm` | `CommandProtocol::arm()` / `disarm()` |
| `rtl` | `CommandProtocol::return_to_launch()` |
| `ms <seq>` | `MissionProtocol::set_current_with_command()` |
| `pw` | `ParameterProtocol::write_by_name()` (type from cache) |

## Serial I/O

This example uses a small cross-platform `SerialMavlinkLink` (Win32 `CreateFile` on Windows; `termios` on Linux/macOS). No third-party serial library is required.

Port enumeration:

- **Windows** — `HKEY_LOCAL_MACHINE\HARDWARE\DEVICEMAP\SERIALCOMM`
- **Linux** — `/dev/ttyUSB*`, `/dev/ttyACM*`
- **macOS** — `/dev/cu.*`, `/dev/tty.usb*`

## Layout

```
examples/cpp/
  CMakeLists.txt
  include/          # GCS helpers (serial link, port picker, mission sample)
  src/sitl_gcs.cpp  # Main CLI application
```

Generated protocol runtime is compiled from `../../generated/cpp/protocols/`.
