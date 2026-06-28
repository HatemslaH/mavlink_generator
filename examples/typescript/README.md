# Real TypeScript GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/ts` and the transport-agnostic protocol classes.

Synthetic round-trip demos remain under `generated/ts/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the TypeScript bindings (at least `rt_rc`):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang type-script
   ```

2. [Node.js](https://nodejs.org/) 20+

3. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows, `/dev/ttyUSB0` on Linux)

## Setup

```bash
cd examples/typescript
npm install
```

## Run

```bash
npm start
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Use `--baud` if your link uses another rate:

```bash
npm start -- --baud 115200
```

## Flow

1. **Bootstrap** — `MavlinkGcs.connect` + heartbeats, `waitForVehicle`.
2. **Parameters** — `ParameterProtocol.fetchAll({ onProgress })` with optional `cancel`.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### CLI commands

| Command | API used |
|---------|----------|
| `cancel` | `MavlinkCancellationToken` on long param/mission ops |
| `att [sec]` | `listenMessage` + `setMessageInterval` |
| `arm` / `disarm` | `CommandProtocol.arm()` / `disarm()` |
| `rtl` | `CommandProtocol.returnToLaunch()` |
| `ms <seq>` | `MissionProtocol.setCurrentWithCommand()` |
| `pw` | `ParameterProtocol.writeByName()` (type from cache) |

## Packages

- [`serialport`](https://www.npmjs.com/package/serialport) — cross-platform serial I/O (Windows COM, Linux `/dev/tty*`, macOS `/dev/cu.*`)
- Generated MAVLink bindings — `../../generated/ts` (regenerate via `cargo run` above)

Imports use `generated/ts/dialects/rt_rc.ts` and individual `generated/ts/protocols/*.ts` modules (Node ESM does not reliably resolve `export *` barrel re-exports from the generated entry files).

> **Note:** SITL and most flight controllers expose MAVLink over a **virtual COM port** (`COM23`, `/dev/ttyUSB0`, etc.), so a serial library is the correct transport choice here.
