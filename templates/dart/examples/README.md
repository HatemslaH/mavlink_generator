# Dart examples

Generated usage examples for the MAVLink Dart bindings in the parent directory.

Shared helpers:

- `common.dart` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.dart` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.dart` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.dart` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.dart` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.dart` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/dart/` at the project root.

## Protocol class examples

Transport-agnostic protocol implementations live in `../lib/protocols/`. Import them via `mavlink_protocols.dart`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.dart` | `MissionProtocol`, `MissionServer` | Upload/download with `onProgress`, `setCurrentWithCommand` |
| `{dialect}_protocol_parameters.dart` | `ParameterProtocol`, `ParameterServer` | `fetchAll(onProgress:)`, `writeByName`, parameter cache |
| `{dialect}_protocol_command.dart` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.dart` | `HeartbeatMonitor`, `HeartbeatPublisher` | `waitForVehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.dart` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.dart` | `MavlinkSession` | `listenMessage<T>`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/dart` directory:

```bash
dart run examples/rt_rc_heartbeat.dart
dart run examples/rt_rc_mission_upload.dart
dart run examples/rt_rc_request_telemetry.dart
dart run examples/rt_rc_request_parameters.dart
dart run examples/rt_rc_protocol_mission.dart
dart run examples/rt_rc_protocol_parameters.dart
dart run examples/rt_rc_protocol_command.dart
dart run examples/rt_rc_protocol_heartbeat.dart
dart run examples/rt_rc_protocol_vehicle.dart
dart run examples/rt_rc_protocol_subscribe.dart
```

Replace `rt_rc` with the dialect name you generated.
