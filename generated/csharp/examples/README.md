# C# examples

Generated usage examples for the MAVLink C# bindings in the parent directory.

Shared helpers live in `common.cs` (GCS/drone identities, framing, param id encoding).

| File | MAVLink service | Protocol flow |
|------|-----------------|---------------|
| `{dialect}_heartbeat.cs` | Heartbeat | Create → serialize → parse |
| `{dialect}_mission_upload.cs` | [Mission](https://mavlink.io/en/services/mission.html) | MISSION_COUNT → MISSION_REQUEST → MISSION_ITEM → MISSION_ACK |
| `{dialect}_request_telemetry.cs` | [Command](https://mavlink.io/en/services/command.html) | COMMAND_LONG (SET_MESSAGE_INTERVAL / REQUEST_MESSAGE) → ATTITUDE |
| `{dialect}_request_parameters.cs` | [Parameter](https://mavlink.io/en/services/parameter.html) | PARAM_REQUEST_LIST / PARAM_REQUEST_READ → PARAM_VALUE |

All examples are **virtual**: no real link, only valid MAVLink frames serialized and parsed locally.

## Run

From the `generated/csharp` directory:

```bash
dotnet run --project examples/rt_rc_heartbeat.csproj
dotnet run --project examples/rt_rc_mission_upload.csproj
dotnet run --project examples/rt_rc_request_telemetry.csproj
dotnet run --project examples/rt_rc_request_parameters.csproj
```

Replace `rt_rc` with the dialect name you generated.
