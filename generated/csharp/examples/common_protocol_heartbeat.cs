/// <summary>
/// Heartbeat protocol example for the `common` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

var gcsPublisher = new HeartbeatPublisher(
    link.Gcs,
    HeartbeatTemplates.Gcs(dialect.Version),
    TimeSpan.FromMilliseconds(500));

var dronePublisher = new HeartbeatPublisher(
    link.Drone,
    HeartbeatTemplates.Autopilot(dialect.Version),
    TimeSpan.FromMilliseconds(500));

var gcsMonitor = new HeartbeatMonitor(link.Gcs, TimeSpan.FromSeconds(2));

gcsMonitor.Start();
gcsPublisher.Start();
dronePublisher.Start();

var vehicle = await gcsMonitor.WaitForVehicleAsync(
    excludeSystemIds: new HashSet<byte> { ProtocolsCommon.GcsSystemId },
    timeout: TimeSpan.FromSeconds(5));
Console.WriteLine($"Vehicle discovered: {vehicle}");
Console.WriteLine($"Drone online: {gcsMonitor.IsOnline(vehicle)}");
var state = gcsMonitor.StateFor(vehicle);
if (state is not null)
{
    Console.WriteLine(
        $"Drone heartbeat: type={state.Heartbeat.Type} status={state.Heartbeat.SystemStatus}");
}

dronePublisher.Stop();
await Task.Delay(TimeSpan.FromMilliseconds(2500));
Console.WriteLine($"Drone online after stop: {gcsMonitor.IsOnline(vehicle)}");

await gcsMonitor.StopAsync();
gcsPublisher.Stop();

await ProtocolsCommon.CloseVirtualLinkAsync(link);
