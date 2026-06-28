# C# examples

Generated usage examples for the MAVLink C# bindings in the parent directory.

Shared helpers:

- `common.cs` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.cs` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.cs` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.cs` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.cs` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.cs` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/csharp/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Import them via `MavlinkProtocols` (barrel namespace).

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.cs` | `MissionProtocol`, `MissionServer` | Upload/download with progress, `SetCurrentWithCommand` |
| `{dialect}_protocol_parameters.cs` | `ParameterProtocol`, `ParameterServer` | `FetchAll`, `WriteByName`, parameter cache |
| `{dialect}_protocol_command.cs` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.cs` | `HeartbeatMonitor`, `HeartbeatPublisher` | `WaitForVehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.cs` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.cs` | `MavlinkSession` | `ListenMessage`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/csharp` directory:

```bash
dotnet run --project examples/rt_rc_heartbeat.csproj
dotnet run --project examples/rt_rc_mission_upload.csproj
dotnet run --project examples/rt_rc_request_telemetry.csproj
dotnet run --project examples/rt_rc_request_parameters.csproj
dotnet run --project examples/rt_rc_protocol_mission.csproj
dotnet run --project examples/rt_rc_protocol_parameters.csproj
dotnet run --project examples/rt_rc_protocol_command.csproj
dotnet run --project examples/rt_rc_protocol_heartbeat.csproj
dotnet run --project examples/rt_rc_protocol_vehicle.csproj
dotnet run --project examples/rt_rc_protocol_subscribe.csproj
```

Replace `rt_rc` with the dialect name you generated.
