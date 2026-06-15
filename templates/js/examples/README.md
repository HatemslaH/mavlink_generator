# JavaScript examples

Generated usage examples for the MAVLink JavaScript bindings in the parent directory.

Shared helpers live in `common.js` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.js` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.js` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.js` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.js` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

## Run

From the `generated/js` directory:

```bash
node examples/rt_rc_heartbeat.js
node examples/rt_rc_mission_upload.js
node examples/rt_rc_request_telemetry.js
node examples/rt_rc_request_parameters.js
```

Replace `rt_rc` with the dialect name you generated.
