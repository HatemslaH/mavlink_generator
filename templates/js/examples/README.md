# JavaScript examples

Generated usage examples for the MAVLink JavaScript bindings in the parent directory.

Shared helpers:

- `common.js` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.js` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.js` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.js` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.js` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.js` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/javascript/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Import them via `mavlink_protocols.js`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.js` | `MissionProtocol`, `MissionServer` | Upload/download with `onProgress`, `setCurrentWithCommand` |
| `{dialect}_protocol_parameters.js` | `ParameterProtocol`, `ParameterServer` | `fetchAll(onProgress:)`, `writeByName`, parameter cache |
| `{dialect}_protocol_command.js` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.js` | `HeartbeatMonitor`, `HeartbeatPublisher` | `waitForVehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.js` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.js` | `MavlinkSession` | `listenMessage`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/js` directory:

```bash
node examples/rt_rc_heartbeat.js
node examples/rt_rc_mission_upload.js
node examples/rt_rc_request_telemetry.js
node examples/rt_rc_request_parameters.js
node examples/rt_rc_protocol_mission.js
node examples/rt_rc_protocol_parameters.js
node examples/rt_rc_protocol_command.js
node examples/rt_rc_protocol_heartbeat.js
node examples/rt_rc_protocol_vehicle.js
node examples/rt_rc_protocol_subscribe.js
```

Replace `rt_rc` with the dialect name you generated.
