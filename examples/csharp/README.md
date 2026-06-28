# Real C# GCS example (SITL / serial)

Interactive ground-station sample that talks to a real MAVLink link (USB serial / virtual COM port) using **generated** bindings from `generated/csharp` and the transport-agnostic protocol classes.

Synthetic round-trip demos remain under `generated/csharp/examples/`. This folder is for end-to-end use with SITL or hardware.

## Prerequisites

1. Generate the C# bindings (common dialect for SITL; `rt_rc` C# output currently has duplicate-message issues in large dialects):

   ```bash
   cargo run -- --input mavlink/message_definitions/v1.0/common.xml --lang c-sharp
   ```

2. [.NET SDK](https://dotnet.microsoft.com/download) 8.0+

3. SITL or autopilot on a serial port (e.g. virtual `COM23` on Windows)

## Setup

```bash
cd examples/csharp
dotnet restore
```

## Run

```bash
dotnet run
```

On start the app lists available serial ports and asks you to pick one. Default baud is **57600** (common for ArduPilot SITL). Use `--baud` if your link uses another rate:

```bash
dotnet run -- --baud 115200
```

## Flow

1. **Bootstrap** — `MavlinkGcs.Connect` + heartbeats, `WaitForVehicleAsync`.
2. **Parameters** — `ParameterProtocol.FetchAllAsync(onProgress:)` with optional `cancel`.
3. **CLI** — interactive commands for parameters, mission, commands, and live ATTITUDE stream.

Type `help` in the CLI for the full command list.

### CLI commands

| Command | API used |
|---------|----------|
| `cancel` | `MavlinkCancellationToken` on long param/mission ops |
| `att [sec]` | `ListenMessage<Attitude>` + `SetMessageIntervalAsync` |
| `arm` / `disarm` | `CommandProtocol.ArmAsync()` / `DisarmAsync()` |
| `rtl` | `CommandProtocol.ReturnToLaunchAsync()` |
| `ms <seq>` | `MissionProtocol.SetCurrentWithCommandAsync()` |
| `pw` | `ParameterProtocol.WriteByNameAsync()` (type from cache) |

## Packages

- [`System.IO.Ports`](https://www.nuget.org/packages/System.IO.Ports) — serial/COM I/O (cross-platform; COM ports on Windows, `/dev/tty*` on Linux)
- `Mavlink` — project reference on `../../generated/csharp`
