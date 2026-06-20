# C examples

Generated usage examples for the MAVLink C bindings in the parent directory.

Shared helpers:

- `common.h` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.h` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.c` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.c` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.c` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.c` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/c/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Include them via `mavlink_protocols.h`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.c` | `mission_protocol`, `mission_server` | Upload/download with progress, `set_current_with_command` |
| `{dialect}_protocol_parameters.c` | `parameter_protocol`, `parameter_server` | `fetch_all`, `write_by_name`, parameter cache |
| `{dialect}_protocol_command.c` | `command_protocol`, `command_server` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.c` | `heartbeat_monitor`, `heartbeat_publisher` | `wait_for_vehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.c` | `mavlink_gcs`, `mavlink_vehicle_client` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.c` | `mavlink_session` | Message subscription, typed telemetry |

Swap `virtual_mavlink_bus` for your own `mavlink_link` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Build and run

From the `generated/c` directory:

```bash
gcc -std=c11 -I. examples/rt_rc_heartbeat.c -o rt_rc_heartbeat
./rt_rc_heartbeat

gcc -std=c11 -I. examples/rt_rc_mission_upload.c -o rt_rc_mission_upload
./rt_rc_mission_upload

gcc -std=c11 -I. examples/rt_rc_request_telemetry.c -o rt_rc_request_telemetry
./rt_rc_request_telemetry

gcc -std=c11 -I. examples/rt_rc_request_parameters.c -o rt_rc_request_parameters
./rt_rc_request_parameters

gcc -std=c11 -I. examples/rt_rc_protocol_mission.c -o rt_rc_protocol_mission
./rt_rc_protocol_mission

gcc -std=c11 -I. examples/rt_rc_protocol_parameters.c -o rt_rc_protocol_parameters
./rt_rc_protocol_parameters

gcc -std=c11 -I. examples/rt_rc_protocol_command.c -o rt_rc_protocol_command
./rt_rc_protocol_command

gcc -std=c11 -I. examples/rt_rc_protocol_heartbeat.c -o rt_rc_protocol_heartbeat
./rt_rc_protocol_heartbeat

gcc -std=c11 -I. examples/rt_rc_protocol_vehicle.c -o rt_rc_protocol_vehicle
./rt_rc_protocol_vehicle

gcc -std=c11 -I. examples/rt_rc_protocol_subscribe.c -o rt_rc_protocol_subscribe
./rt_rc_protocol_subscribe
```

Replace `rt_rc` with the dialect name you generated.
