# Rust examples

Generated usage examples for the MAVLink Rust bindings in the parent directory.

Shared helpers:

- `common.rs` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.rs` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.rs` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.rs` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.rs` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.rs` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/rust/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../src/protocols/`. Import them via the `mavlink_protocols` module.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.rs` | `MissionProtocol`, `MissionServer` | Upload/download with progress, `set_current_with_command` |
| `{dialect}_protocol_parameters.rs` | `ParameterProtocol`, `ParameterServer` | `fetch_all`, `write_by_name`, parameter cache |
| `{dialect}_protocol_command.rs` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.rs` | `HeartbeatMonitor`, `HeartbeatPublisher` | `wait_for_vehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.rs` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.rs` | `MavlinkSession` | `listen_message`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/rust` directory:

```bash
cargo run --example rt_rc_heartbeat
cargo run --example rt_rc_mission_upload
cargo run --example rt_rc_request_telemetry
cargo run --example rt_rc_request_parameters
cargo run --example rt_rc_protocol_mission
cargo run --example rt_rc_protocol_parameters
cargo run --example rt_rc_protocol_command
cargo run --example rt_rc_protocol_heartbeat
cargo run --example rt_rc_protocol_vehicle
cargo run --example rt_rc_protocol_subscribe
```

Replace `rt_rc` with the dialect name you generated.
