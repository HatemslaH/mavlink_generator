/// <summary>
/// MavlinkGcs / MavlinkVehicleClient facade example for `rt_rc`.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectRt_rc();
var bus = new VirtualMavlinkBus();
var gcsLink = bus.CreateEndpoint();
var droneLink = bus.CreateEndpoint();

await using var gcs = MavlinkGcs.Connect(dialect, gcsLink);

var droneSession = new MavlinkSession(
    dialect,
    droneLink,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var dronePublisher = new HeartbeatPublisher(
    droneSession,
    HeartbeatTemplates.Autopilot(dialect.Version),
    TimeSpan.FromMilliseconds(500));

await using var parameterServer = new ParameterServer(
    droneSession,
    new Dictionary<string, ParamStoreEntry>
    {
        ["SYSID_THISMAV"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
    });

await using var commandServer = new CommandServer(droneSession);

gcs.Start();
dronePublisher.Start();

var client = await gcs.WaitForVehicleAsync(
    excludeSystemIds: new HashSet<byte> { ProtocolsCommon.GcsSystemId });
Console.WriteLine($"Connected to vehicle {client.Vehicle}");

var parameters = await client.Parameters.FetchAllAsync();
Console.WriteLine($"Vehicle has {parameters.Count} parameters");

var ack = await client.Command.RequestMessageAsync(Heartbeat.MsgId);
Console.WriteLine($"REQUEST_MESSAGE ack: {ack.Result}");

dronePublisher.Stop();
await droneSession.CloseAsync();
await gcs.CloseAsync();
await bus.CloseAllAsync();
