/// <summary>
/// Mission protocol example for the `common` dialect.
/// </summary>
/// <remarks>Uses <see cref="VirtualMavlinkBus"/> via <see cref="ProtocolsCommon.CreateVirtualLink"/>.</remarks>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var missionServer = new MissionServer(link.Drone);
await using var commandServer = new CommandServer(link.Drone);
var missionProtocol = new MissionProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var plan = new[]
{
    MissionItems.Waypoint(
        0,
        47.397742,
        8.545594,
        50,
        ProtocolsCommon.DroneSystemId,
        ProtocolsCommon.DroneComponentId),
    MissionItems.Waypoint(
        1,
        47.398000,
        8.546000,
        50,
        ProtocolsCommon.DroneSystemId,
        ProtocolsCommon.DroneComponentId),
};

var uploadResult = await missionProtocol.UploadAsync(
    plan,
    onProgress: (sent, total, item) =>
        Console.WriteLine($"Upload progress {sent}/{total} seq={item.Seq} cmd={item.Command}"));
Console.WriteLine($"Mission upload result: {uploadResult}");
Console.WriteLine($"Vehicle stored {missionServer.Items.Count} items");

var downloaded = await missionProtocol.DownloadAsync(
    onProgress: (received, total, item) =>
        Console.WriteLine($"Download progress {received}/{total} seq={item.Seq}"));
Console.WriteLine($"Downloaded {downloaded.Count} mission items");

var commandProtocol = new CommandProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);
var setCurrent = await missionProtocol.SetCurrentWithCommandAsync(
    0,
    command: commandProtocol);
Console.WriteLine($"Set current seq={setCurrent.Sequence} ack={setCurrent.CommandAck?.Result}");

var clearResult = await missionProtocol.ClearAsync();
Console.WriteLine($"Mission clear result: {clearResult}");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
