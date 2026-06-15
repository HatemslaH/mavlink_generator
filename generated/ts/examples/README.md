# TypeScript examples

Generated usage examples for the MAVLink TypeScript bindings in the parent directory.

Shared helpers live in `common.ts` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.ts` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.ts` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.ts` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.ts` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

## Run

From the `generated/ts` directory (requires [tsx](https://github.com/privatenumber/tsx) or similar):

```bash
npx tsx examples/rt_rc_heartbeat.ts
npx tsx examples/rt_rc_mission_upload.ts
npx tsx examples/rt_rc_request_telemetry.ts
npx tsx examples/rt_rc_request_parameters.ts
```

Replace `rt_rc` with the dialect name you generated.
