# Python examples

Generated usage examples for the MAVLink Python bindings in the parent directory.

Shared helpers:

- `common.py` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.py` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.py` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.py` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.py` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.py` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/python/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Import them via `mavlink_protocols.py`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.py` | `MissionProtocol`, `MissionServer` | Upload/download with `on_progress`, `set_current_with_command` |
| `{dialect}_protocol_parameters.py` | `ParameterProtocol`, `ParameterServer` | `fetch_all(on_progress=)`, `write_by_name`, parameter cache |
| `{dialect}_protocol_command.py` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.py` | `HeartbeatMonitor`, `HeartbeatPublisher` | `wait_for_vehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.py` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.py` | `MavlinkSession` | `listen_message`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Run

From the `generated/py` directory:

```bash
python examples/rt_rc_heartbeat.py
python examples/rt_rc_mission_upload.py
python examples/rt_rc_request_telemetry.py
python examples/rt_rc_request_parameters.py
python examples/rt_rc_protocol_mission.py
python examples/rt_rc_protocol_parameters.py
python examples/rt_rc_protocol_command.py
python examples/rt_rc_protocol_heartbeat.py
python examples/rt_rc_protocol_vehicle.py
python examples/rt_rc_protocol_subscribe.py
```

Replace `rt_rc` with the dialect name you generated.
