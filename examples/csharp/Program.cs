using System.Globalization;
using Mavlink;
using Mavlink.Dialects;
using MavlinkSitlGcs;

var baudRate = PortPicker.ParseBaudRate(args);
var portName = PortPicker.PickSerialPort();

Console.WriteLine();
Console.WriteLine($"Opening {portName} @ {baudRate} baud...");

var dialect = new MavlinkDialectCommon();
var link = SerialMavlinkLink.Open(portName, baudRate);
var gcs = MavlinkGcs.Connect(
    dialect,
    link,
    systemId: GcsIdentity.SystemId,
    componentId: GcsIdentity.ComponentId);

gcs.Start();
Console.WriteLine("Publishing GCS heartbeats, waiting for vehicle...");

MavlinkVehicleClient client;
try
{
    client = await gcs.WaitForVehicleAsync(
        excludeSystemIds: new HashSet<byte> { GcsIdentity.SystemId },
        timeout: TimeSpan.FromSeconds(60));
}
catch (MavlinkTimeoutException)
{
    throw new InvalidOperationException(
        $"No vehicle heartbeat within 60 s. Check port, baud (current: {baudRate}; try --baud 115200), and SITL.");
}

var vehicle = client.Vehicle;
var vehicleState = gcs.HeartbeatMonitor.StateFor(vehicle);
Console.WriteLine($"Vehicle online: {vehicle}");
if (vehicleState is not null)
{
    Console.WriteLine(
        $"  type={vehicleState.Heartbeat.Type} " +
        $"autopilot={vehicleState.Heartbeat.Autopilot} " +
        $"status={vehicleState.Heartbeat.SystemStatus}");
}

var ctx = new GcsContext(gcs, vehicle, client);

Console.WriteLine();
Console.WriteLine("=== Phase 2: parameter sync ===");
await FetchAllParametersAsync(ctx);

Console.WriteLine();
Console.WriteLine("=== Interactive CLI ===");
await RunCliAsync(ctx);

Console.WriteLine("Shutting down...");
ctx.OperationCancel?.Cancel();
await gcs.CloseAsync();

static async Task FetchAllParametersAsync(GcsContext ctx)
{
    var cancel = new MavlinkCancellationToken();
    ctx.OperationCancel = cancel;

    Console.WriteLine("[parameters] waiting for PARAM_VALUE stream...");
    var entries = await ctx.Parameters.FetchAllAsync(
        cancel: cancel,
        onProgress: (entry, received, expected) =>
        {
            if (received == 1)
            {
                Console.WriteLine($"[parameters] expecting {expected} parameters");
            }

            Console.WriteLine(
                $"[parameters] {received}/{expected} {entry.Id}={entry.Value} ({entry.Type})");
        });
    Console.WriteLine(
        $"[parameters] complete ({entries.Count} total, cache={ctx.Parameters.Cache.Count})");
}

static async Task RunCliAsync(GcsContext ctx)
{
    PrintHelp();

    while (true)
    {
        Console.Write("gcs> ");
        var line = Console.ReadLine();
        if (line is null)
        {
            break;
        }

        var trimmed = line.Trim();
        if (trimmed.Length == 0)
        {
            continue;
        }

        var parts = trimmed.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        var command = parts[0].ToLowerInvariant();

        try
        {
            switch (command)
            {
                case "h":
                case "help":
                    PrintHelp();
                    break;
                case "q":
                case "quit":
                case "exit":
                    return;
                case "hb":
                    PrintHeartbeatStatus(ctx);
                    break;
                case "cancel":
                    CancelOperation(ctx);
                    break;
                case "p":
                case "params":
                    await FetchAllParametersAsync(ctx);
                    break;
                case "pr":
                    await ReadParameterAsync(ctx, parts);
                    break;
                case "pw":
                    await WriteParameterAsync(ctx, parts);
                    break;
                case "mu":
                    await UploadMissionAsync(ctx);
                    break;
                case "md":
                    await DownloadMissionAsync(ctx);
                    break;
                case "mc":
                    await ClearMissionAsync(ctx);
                    break;
                case "ms":
                    await SetMissionCurrentAsync(ctx, parts);
                    break;
                case "rm":
                    await RequestMessageAsync(ctx, parts);
                    break;
                case "si":
                    await SetMessageIntervalAsync(ctx, parts);
                    break;
                case "att":
                    await StreamAttitudeAsync(ctx, parts);
                    break;
                case "arm":
                    await ArmAsync(ctx, parts);
                    break;
                case "disarm":
                    await DisarmAsync(ctx, parts);
                    break;
                case "rtl":
                    await ReturnToLaunchAsync(ctx);
                    break;
                default:
                    Console.WriteLine($"Unknown command: {command} (type help)");
                    break;
            }
        }
        catch (MavlinkCancelledException)
        {
            Console.WriteLine("Operation cancelled.");
        }
        catch (Exception error)
        {
            Console.WriteLine($"Error: {error.Message}");
        }

        Console.WriteLine();
    }
}

static void PrintHelp()
{
    Console.WriteLine("Commands:");
    Console.WriteLine("  help              Show this help");
    Console.WriteLine("  hb                Heartbeat / link status");
    Console.WriteLine("  cancel            Cancel in-flight params/mission operation");
    Console.WriteLine("  params            Request full parameter list (with progress)");
    Console.WriteLine("  pr <name>         Read one parameter by name");
    Console.WriteLine("  pw <name> <value> Write parameter (type from cache or REAL32)");
    Console.WriteLine("  mu                Upload hardcoded sample mission");
    Console.WriteLine("  md                Download mission from vehicle");
    Console.WriteLine("  mc                Clear onboard mission");
    Console.WriteLine("  ms <seq>          Set active mission item (mission + command)");
    Console.WriteLine("  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)");
    Console.WriteLine("  si <msgId> <us>   Set message interval (microseconds)");
    Console.WriteLine("  att [seconds]     Stream ATTITUDE via onMessage (default 5 s)");
    Console.WriteLine("  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)");
    Console.WriteLine("  disarm [force]    Disarm motors");
    Console.WriteLine("  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH");
    Console.WriteLine("  quit              Exit");
}

static void CancelOperation(GcsContext ctx)
{
    var token = ctx.OperationCancel;
    if (token is null || token.IsCancelled)
    {
        Console.WriteLine("[cancel] no active cancellable operation");
        return;
    }

    token.Cancel();
    Console.WriteLine("[cancel] signalled");
}

static void PrintHeartbeatStatus(GcsContext ctx)
{
    var node = ctx.Vehicle;
    var online = ctx.HeartbeatMonitor.IsOnline(node);
    var state = ctx.HeartbeatMonitor.StateFor(node);

    Console.WriteLine($"[heartbeat] vehicle {node} online={online}");
    if (state is not null)
    {
        Console.WriteLine(
            $"  last={state.Age.TotalMilliseconds:F0}ms ago " +
            $"type={state.Heartbeat.Type} status={state.Heartbeat.SystemStatus}");
    }
    else
    {
        Console.WriteLine("  no heartbeat received yet");
    }
}

static async Task ReadParameterAsync(GcsContext ctx, string[] parts)
{
    if (parts.Length < 2)
    {
        Console.WriteLine("Usage: pr <name>");
        return;
    }

    var name = parts[1];
    Console.WriteLine($"[parameters] reading {name}...");
    var entry = await ctx.Parameters.ReadByNameAsync(name);
    Console.WriteLine(
        $"[parameters] {name}={entry.Value} ({entry.Type}, index {entry.Index}/{entry.Count})");
}

static async Task WriteParameterAsync(GcsContext ctx, string[] parts)
{
    if (parts.Length < 3)
    {
        Console.WriteLine("Usage: pw <name> <value>");
        return;
    }

    var name = parts[1];
    var rawValue = parts[2];
    var cachedType = ctx.Parameters.TypeForName(name);
    var type = cachedType ?? MavParamType.MAV_PARAM_TYPE_REAL32;
    var value = ParseParamValue(rawValue, type);

    Console.WriteLine($"[parameters] writing {name}={value} ({type})...");
    var entry = await ctx.Parameters.WriteByNameAsync(name, value);
    Console.WriteLine($"[parameters] ack {name}={entry.Value} ({entry.Type})");
}

static decimal ParseParamValue(string raw, MavParamType type) =>
    type switch
    {
        MavParamType.MAV_PARAM_TYPE_INT8 or
        MavParamType.MAV_PARAM_TYPE_INT16 or
        MavParamType.MAV_PARAM_TYPE_INT32 or
        MavParamType.MAV_PARAM_TYPE_UINT8 or
        MavParamType.MAV_PARAM_TYPE_UINT16 or
        MavParamType.MAV_PARAM_TYPE_UINT32 =>
            int.Parse(raw, CultureInfo.InvariantCulture),
        _ => decimal.Parse(raw, CultureInfo.InvariantCulture),
    };

static async Task UploadMissionAsync(GcsContext ctx)
{
    var plan = SampleMission.Build(ctx.TargetSystem, ctx.TargetComponent);
    var cancel = new MavlinkCancellationToken();
    ctx.OperationCancel = cancel;

    Console.WriteLine($"[mission] uploading {plan.Count} hardcoded items...");
    var result = await ctx.Mission.UploadAsync(
        plan,
        cancel: cancel,
        onProgress: (sent, total, item) =>
        {
            Console.WriteLine(
                $"[mission upload] {sent}/{total} seq={item.Seq} {SampleMission.Describe(item)}");
        });
    Console.WriteLine($"[mission] upload finished: {result}");
}

static async Task DownloadMissionAsync(GcsContext ctx)
{
    var cancel = new MavlinkCancellationToken();
    ctx.OperationCancel = cancel;

    var items = await ctx.Mission.DownloadAsync(
        cancel: cancel,
        onProgress: (received, total, item) =>
        {
            Console.WriteLine(
                $"[mission download] {received}/{total} {SampleMission.Describe(item)}");
        });
    Console.WriteLine("[mission] on vehicle:");
    foreach (var item in items)
    {
        Console.WriteLine($"  {SampleMission.Describe(item)}");
    }
}

static async Task ClearMissionAsync(GcsContext ctx)
{
    Console.WriteLine("[mission] sending MISSION_CLEAR_ALL...");
    var result = await ctx.Mission.ClearAsync();
    Console.WriteLine($"[mission] clear result: {result}");
}

static async Task SetMissionCurrentAsync(GcsContext ctx, string[] parts)
{
    if (parts.Length < 2)
    {
        Console.WriteLine("Usage: ms <seq>");
        return;
    }

    var seq = int.Parse(parts[1], CultureInfo.InvariantCulture);
    Console.WriteLine($"[mission] set current seq={seq} (mission + command)...");
    var result = await ctx.Mission.SetCurrentWithCommandAsync(seq, command: ctx.Command);
    Console.WriteLine(
        $"[mission] seq={result.Sequence} command ack={result.CommandAck?.Result.ToString() ?? "n/a"}");
}

static async Task RequestMessageAsync(GcsContext ctx, string[] parts)
{
    if (parts.Length < 2)
    {
        Console.WriteLine($"Usage: rm <msgId>  (e.g. rm {Attitude.MsgId} for ATTITUDE)");
        return;
    }

    var msgId = int.Parse(parts[1], CultureInfo.InvariantCulture);
    Console.WriteLine($"[command] REQUEST_MESSAGE id={msgId}");
    var ack = await ctx.Command.RequestMessageAsync(msgId);
    Console.WriteLine($"[command] ack: {ack.Result}");

    if (msgId == Attitude.MsgId)
    {
        Console.WriteLine("[telemetry] waiting for ATTITUDE...");
        var attitude = await ctx.Session.WaitForMessageTypeAsync<Attitude>(
            fromSystemId: ctx.TargetSystem,
            timeout: TimeSpan.FromSeconds(5));
        Console.WriteLine(
            $"[telemetry] roll={attitude.Roll} pitch={attitude.Pitch} yaw={attitude.Yaw}");
    }
}

static async Task SetMessageIntervalAsync(GcsContext ctx, string[] parts)
{
    if (parts.Length < 3)
    {
        Console.WriteLine("Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)");
        return;
    }

    var msgId = int.Parse(parts[1], CultureInfo.InvariantCulture);
    var intervalUs = int.Parse(parts[2], CultureInfo.InvariantCulture);
    Console.WriteLine($"[command] SET_MESSAGE_INTERVAL id={msgId} interval={intervalUs} us");
    var ack = intervalUs == 0
        ? await ctx.Command.StopMessageIntervalAsync(msgId)
        : await ctx.Command.SetMessageIntervalAsync(msgId, intervalUs);
    Console.WriteLine($"[command] ack: {ack.Result}");
}

static async Task StreamAttitudeAsync(GcsContext ctx, string[] parts)
{
    var seconds = parts.Length >= 2
        ? int.Parse(parts[1], CultureInfo.InvariantCulture)
        : 5;
    Console.WriteLine(
        $"[telemetry] streaming ATTITUDE for {seconds}s (subscribe + interval)...");

    await ctx.Command.SetMessageIntervalAsync(Attitude.MsgId, 100000);

    var count = 0;
    var subscription = ctx.Session.ListenMessage<Attitude>(
        (attitude, _) =>
        {
            count++;
            Console.WriteLine(
                $"[attitude] #{count} roll={attitude.Roll:F3} " +
                $"pitch={attitude.Pitch:F3} yaw={attitude.Yaw:F3}");
        },
        fromSystemId: ctx.TargetSystem);

    try
    {
        await Task.Delay(TimeSpan.FromSeconds(seconds));
    }
    finally
    {
        subscription.Cancel();
    }
    await ctx.Command.StopMessageIntervalAsync(Attitude.MsgId);
    Console.WriteLine($"[telemetry] received {count} ATTITUDE messages");
}

static async Task ArmAsync(GcsContext ctx, string[] parts)
{
    var force = parts.Length >= 2 && parts[1].Equals("force", StringComparison.OrdinalIgnoreCase);
    Console.WriteLine($"[command] ARM{(force ? " (force)" : "")}...");
    var ack = await ctx.Command.ArmAsync(force: force);
    Console.WriteLine($"[command] ack: {ack.Result}");
}

static async Task DisarmAsync(GcsContext ctx, string[] parts)
{
    var force = parts.Length >= 2 && parts[1].Equals("force", StringComparison.OrdinalIgnoreCase);
    Console.WriteLine($"[command] DISARM{(force ? " (force)" : "")}...");
    var ack = await ctx.Command.DisarmAsync(force: force);
    Console.WriteLine($"[command] ack: {ack.Result}");
}

static async Task ReturnToLaunchAsync(GcsContext ctx)
{
    Console.WriteLine("[command] RETURN_TO_LAUNCH...");
    var ack = await ctx.Command.ReturnToLaunchAsync();
    Console.WriteLine($"[command] ack: {ack.Result}");
}
