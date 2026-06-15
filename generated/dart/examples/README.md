# Dart examples

Generated usage examples for the MAVLink Dart bindings in the parent directory.

Shared helpers live in `common.dart` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.dart` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.dart` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.dart` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.dart` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

## Run

From the `generated/dart` directory:

```bash
dart run examples/rt_rc_heartbeat.dart
dart run examples/rt_rc_mission_upload.dart
dart run examples/rt_rc_request_telemetry.dart
dart run examples/rt_rc_request_parameters.dart
```

Replace `rt_rc` with the dialect name you generated.
