# C++ examples

Generated usage examples for the MAVLink C++ bindings in the parent directory.

Shared helpers live in `common.hpp` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.cpp` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.cpp` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.cpp` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.cpp` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

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
```

Replace `rt_rc` with the dialect name you generated.
