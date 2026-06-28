# Real Rust GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/rust` and the transport-agnostic protocol classes.

Synthetic round-trip demos remain under `generated/rust/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the Rust bindings (at least `rt_rc`):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang rust
   ```

2. [Rust](https://www.rust-lang.org/) toolchain (edition 2021)

3. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows, `/dev/ttyUSB0` on Linux)

## Setup

```bash
cd examples/rust
cargo build
```

## Run

```bash
cargo run --bin sitl_gcs
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Pass `--baud` if your link uses another rate:

```bash
cargo run --bin sitl_gcs -- --baud 115200
```

## Flow

1. **Bootstrap** — `MavlinkGcs::connect` + heartbeats, `wait_for_vehicle`.
2. **Parameters** — `ParameterProtocol::fetch_all` with optional progress callback and `cancel`.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### New interactions

| Command | API used |
|---------|----------|
| `cancel` | `MavlinkCancellationToken` on long param/mission ops |
| `att [sec]` | `listen_message::<Attitude>` + `set_message_interval` |
| `arm` / `disarm` | `CommandProtocol::arm()` / `disarm()` |
| `rtl` | `CommandProtocol::return_to_launch()` |
| `ms <seq>` | `MissionProtocol::set_current_with_command()` |
| `pw` | `ParameterProtocol::write_by_name()` (type from cache) |

## Crates

- [`serialport`](https://crates.io/crates/serialport) — cross-platform serial I/O
- [`tokio`](https://crates.io/crates/tokio) — async runtime for protocol waits
- `mavlink` — path dependency on `../../generated/rust`

> **Note:** SITL and most flight controllers expose MAVLink over a **virtual COM port** (`COM23`, `/dev/ttyACM0`, etc.). Raw USB bulk libraries are not needed for this example.
