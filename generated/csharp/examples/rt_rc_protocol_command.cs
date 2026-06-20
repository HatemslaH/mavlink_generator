/// <summary>
/// Command protocol example for the `rt_rc` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectRt_rc();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var commandServer = new CommandServer(
    link.Drone,
    onCommandLong: command =>
    {
        Console.WriteLine(
            $"Vehicle received COMMAND_LONG: {command.Command} " +
            $"p1={command.Param1} p2={command.Param2}");
        return Task.FromResult(MavResult.MAV_RESULT_ACCEPTED);
    });

var commandProtocol = new CommandProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var intervalAck = await commandProtocol.SetMessageIntervalAsync(Attitude.MsgId, 100000);
Console.WriteLine($"SET_MESSAGE_INTERVAL ack: {intervalAck.Result}");

var requestAck = await commandProtocol.RequestMessageAsync(Attitude.MsgId);
Console.WriteLine($"REQUEST_MESSAGE ack: {requestAck.Result}");

var armAck = await commandProtocol.ArmAsync();
Console.WriteLine($"ARM ack: {armAck.Result}");

var disarmAck = await commandProtocol.DisarmAsync();
Console.WriteLine($"DISARM ack: {disarmAck.Result}");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
