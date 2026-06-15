/// <summary>
/// Example for the `rt_rc` dialect: serialize a Heartbeat frame and parse it back.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectRt_rc();

var heartbeat = new Heartbeat(
    CustomMode: 0,
    Type: MavType.MAV_TYPE_QUADROTOR,
    Autopilot: MavAutopilot.MAV_AUTOPILOT_PX4,
    BaseMode: 0,
    SystemStatus: MavState.MAV_STATE_ACTIVE,
    MavlinkVersion: dialect.Version
);

var frame = Common.FrameFromGcs(heartbeat);
var wire = frame.Serialize();
Common.LogFrame("GCS ->", frame);
Console.WriteLine($"Serialized HEARTBEAT ({wire.Length} bytes)");

var parsed = Common.RoundTripMessage(dialect, heartbeat);
if (parsed is Heartbeat parsedHeartbeat)
{
    Console.WriteLine(
        $"Parsed HEARTBEAT type={parsedHeartbeat.Type} status={parsedHeartbeat.SystemStatus}");
}
