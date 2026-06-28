# C++ examples

Generated usage examples for the MAVLink C++ bindings in the parent directory.

Shared helpers:

- `common.hpp` — GCS/drone identities, framing, param id encoding (low-level round-trip demos)
- `protocols_common.hpp` — virtual in-memory link for protocol-class examples

## Low-level message examples

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.cpp` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.cpp` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.cpp` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.cpp` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

These examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

For a **real serial / SITL** interactive GCS sample, see the repository folder `examples/cpp/` at the project root (after Phase 2).

## Protocol class examples

Transport-agnostic protocol implementations live in `../protocols/`. Include them via `mavlink_protocols.hpp`.

| File | Classes | Description |
|------|---------|-------------|
| `{dialect}_protocol_mission.cpp` | `MissionProtocol`, `MissionServer` | Upload/download with progress, `setCurrentWithCommand` |
| `{dialect}_protocol_parameters.cpp` | `ParameterProtocol`, `ParameterServer` | `fetchAll`, `writeByName`, parameter cache |
| `{dialect}_protocol_command.cpp` | `CommandProtocol`, `CommandServer` | Intervals, requests, arm/disarm helpers |
| `{dialect}_protocol_heartbeat.cpp` | `HeartbeatMonitor`, `HeartbeatPublisher` | `waitForVehicle`, connectivity tracking |
| `{dialect}_protocol_vehicle.cpp` | `MavlinkGcs`, `MavlinkVehicleClient` | GCS bootstrap + bundled vehicle protocols |
| `{dialect}_protocol_subscribe.cpp` | `MavlinkSession` | `listenMessage`, typed telemetry subscription |

Swap `VirtualMavlinkBus` for your own `MavlinkLink` (USB serial, UDP, TCP, etc.) — protocol code stays the same.

## Build and run

From the `generated/cpp` directory:

```bash
g++ -std=c++17 -I. examples/rt_rc_heartbeat.cpp -o rt_rc_heartbeat
./rt_rc_heartbeat

g++ -std=c++17 -I. examples/rt_rc_mission_upload.cpp -o rt_rc_mission_upload
./rt_rc_mission_upload

g++ -std=c++17 -I. examples/rt_rc_request_telemetry.cpp -o rt_rc_request_telemetry
./rt_rc_request_telemetry

g++ -std=c++17 -I. examples/rt_rc_request_parameters.cpp -o rt_rc_request_parameters
./rt_rc_request_parameters

g++ -std=c++17 -I. examples/rt_rc_protocol_mission.cpp -o rt_rc_protocol_mission
./rt_rc_protocol_mission

g++ -std=c++17 -I. examples/rt_rc_protocol_parameters.cpp -o rt_rc_protocol_parameters
./rt_rc_protocol_parameters

g++ -std=c++17 -I. examples/rt_rc_protocol_command.cpp -o rt_rc_protocol_command
./rt_rc_protocol_command

g++ -std=c++17 -I. examples/rt_rc_protocol_heartbeat.cpp -o rt_rc_protocol_heartbeat
./rt_rc_protocol_heartbeat

g++ -std=c++17 -I. examples/rt_rc_protocol_vehicle.cpp -o rt_rc_protocol_vehicle
./rt_rc_protocol_vehicle

g++ -std=c++17 -I. examples/rt_rc_protocol_subscribe.cpp -o rt_rc_protocol_subscribe
./rt_rc_protocol_subscribe
```

Replace `rt_rc` with the dialect name you generated.
