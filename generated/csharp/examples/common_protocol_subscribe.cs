/// <summary>
/// Typed message subscription example for the `common` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();
var link = ProtocolsCommon.CreateVirtualLink(dialect);
var vehicle = new MavlinkNode(ProtocolsCommon.DroneSystemId, ProtocolsCommon.DroneComponentId);

var attitudeSamples = new List<Attitude>();
using var subscription = link.Gcs.ListenMessage<Attitude>(
    (message, frame) => attitudeSamples.Add(message),
    fromSystemId: vehicle.SystemId);

await link.Drone.SendAsync(
    new Attitude(
        TimeBootMs: 1000,
        Roll: 0.1f,
        Pitch: -0.05f,
        Yaw: 1.57f,
        Rollspeed: 0,
        Pitchspeed: 0,
        Yawspeed: 0));

await Task.Delay(TimeSpan.FromMilliseconds(50));
subscription.Cancel();

Console.WriteLine($"Received {attitudeSamples.Count} ATTITUDE samples via ListenMessage");
if (attitudeSamples.Count > 0)
{
    var sample = attitudeSamples[0];
    Console.WriteLine($"  roll={sample.Roll} pitch={sample.Pitch} yaw={sample.Yaw}");
}

await ProtocolsCommon.CloseVirtualLinkAsync(link);
