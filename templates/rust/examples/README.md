# Rust examples

Generated usage examples for the MAVLink Rust bindings in the parent directory.

Shared helpers live in `common.rs` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.rs` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.rs` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.rs` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.rs` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

## Run

From the `generated/rust` directory:

```bash
cargo run --example rt_rc_heartbeat
cargo run --example rt_rc_mission_upload
cargo run --example rt_rc_request_telemetry
cargo run --example rt_rc_request_parameters
```

Replace `rt_rc` with the dialect name you generated.
