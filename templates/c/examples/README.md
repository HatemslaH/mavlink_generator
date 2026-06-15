# C examples

Generated usage examples for the MAVLink C bindings in the parent directory.

Shared helpers live in `common.h` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.c` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.c` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.c` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.c` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

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
```

Replace `rt_rc` with the dialect name you generated.
