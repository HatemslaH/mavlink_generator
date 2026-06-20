# TypeScript examples

Generated usage examples for the MAVLink TypeScript bindings in the parent directory.

Shared helpers:

- `common.ts` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.ts` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.ts` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.ts` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.ts` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.ts` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/typescript/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Import them via `mavlink_protocols.ts`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.ts` | `MissionProtocol`, `MissionServer` | Upload/download with `onProgress`, `setCurrentWithCommand` |
| `{dialect}_protocol_parameters.ts` | `ParameterProtocol`, `ParameterServer` | `fetchAll(onProgress:)`, `writeByName`, parameter cache |
| `{dialect}_protocol_command.ts` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.ts` | `HeartbeatMonitor`, `HeartbeatPublisher` | `waitForVehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.ts` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.ts` | `MavlinkSession` | `listenMessage<T>`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/ts` directory (requires [tsx](https://github.com/privatenumber/tsx) or similar):

```bash
npx tsx examples/rt_rc_heartbeat.ts
npx tsx examples/rt_rc_mission_upload.ts
npx tsx examples/rt_rc_request_telemetry.ts
npx tsx examples/rt_rc_request_parameters.ts
npx tsx examples/rt_rc_protocol_mission.ts
npx tsx examples/rt_rc_protocol_parameters.ts
npx tsx examples/rt_rc_protocol_command.ts
npx tsx examples/rt_rc_protocol_heartbeat.ts
npx tsx examples/rt_rc_protocol_vehicle.ts
npx tsx examples/rt_rc_protocol_subscribe.ts
```

Replace `rt_rc` with the dialect name you generated.
