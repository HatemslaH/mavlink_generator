# Real Dart GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/dart` and the transport-agnostic protocol classes.

Synthetic round-trip demos remain under `generated/dart/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the Dart bindings (at least `rt_rc`):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang dart
   ```

2. [Dart SDK](https://dart.dev/get-dart) 3.0+

3. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows)

## Setup

```bash
cd examples/dart
dart pub get
```

## Run

```bash
dart run bin/sitl_gcs.dart
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Use `--baud` if your link uses another rate:

```bash
dart run bin/sitl_gcs.dart --baud 115200
```

## Flow

1. **Bootstrap** — `MavlinkGcs.connect` + heartbeats, `waitForVehicle`.
2. **Parameters** — `ParameterProtocol.fetchAll(onProgress:)` with optional `cancel`.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### New interactions

| Command | API used |
|---------|----------|
| `cancel` | `MavlinkCancellationToken` on long param/mission ops |
| `att [sec]` | `listenMessage<Attitude>` + `setMessageInterval` |
| `arm` / `disarm` | `CommandProtocol.arm()` / `disarm()` |
| `rtl` | `CommandProtocol.returnToLaunch()` |
| `ms <seq>` | `MissionProtocol.setCurrentWithCommand()` |
| `pw` | `ParameterProtocol.writeByName()` (type from cache) |

## Packages

- [`serial_port_win32`](https://pub.dev/packages/serial_port_win32) — pure Dart COM/serial I/O via Win32 API (Windows only; no Flutter)
- `mavlink` — path dependency on `../../generated/dart`

> **Note:** [dart_usb](https://pub.dev/packages/dart_usb) targets raw USB bulk/interrupt transfers. SITL and most flight controllers expose MAVLink over a **virtual COM port** (`COM23`, etc.), so a serial/COM library is the correct choice here.
