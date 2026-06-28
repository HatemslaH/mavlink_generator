# Real C GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/c` and the transport-agnostic protocol layer (callback + blocking `wait_for_message` API).

Synthetic round-trip demos remain under `generated/c/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the C bindings (at least `rt_rc`):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang c
   ```

2. A C11 compiler (GCC, Clang, or MSVC via CMake)

3. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows, `/dev/ttyUSB0` on Linux)

## Build

### Make (GCC / MinGW)

```bash
cd examples/c
make
```

### CMake

```bash
cd examples/c
cmake -B build
cmake --build build
```

The binary is `sitl_gcs` (or `sitl_gcs.exe` on Windows with Make).

## Run

```bash
./sitl_gcs
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Use `--baud` if your link uses another rate:

```bash
./sitl_gcs --baud 115200
```

## Flow

1. **Bootstrap** — `mavlink_gcs_connect` + heartbeats, `mavlink_gcs_wait_for_vehicle`.
2. **Parameters** — `parameter_protocol_fetch_all` with progress callback and optional cancel token.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### CLI commands

| Command | API used |
|---------|----------|
| `cancel` | `mavlink_cancellation_token` on long param/mission ops |
| `att [sec]` | `mavlink_session_listen_message` + `command_protocol_set_message_interval` |
| `arm` / `disarm` | `command_protocol_arm` / `disarm` |
| `rtl` | `command_protocol_return_to_launch` |
| `ms <seq>` | `mission_protocol_set_current_with_command` |
| `pw` | `parameter_protocol_write_by_name` (type from cache) |

## Layout

| Path | Role |
|------|------|
| `src/sitl_gcs.c` | Main entry, CLI, protocol orchestration |
| `src/serial_link.c` | `mavlink_link` over Win32 COM or POSIX termios |
| `src/port_picker.c` | Port enumeration + interactive selection |
| `src/sample_mission.c` | Hardcoded Zurich-area mission plan |
| `include/gcs_context.h` | GCS identity constants and shared context |

Protocol implementations are compiled from `generated/c/protocols/` (not duplicated here).

## Platform notes

- **Windows** — enumerates ports via `SERIALCOMM` registry; opens `\\.\COMx` with Win32 file API.
- **Linux / macOS** — scans `/dev` for `ttyACM*`, `ttyUSB*`, and common USB serial names; uses `termios`.

Only the serial transport is platform-specific; all MAVLink protocol logic uses the generated C protocol layer unchanged.
