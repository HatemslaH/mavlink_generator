/// <summary>
/// Virtual telemetry request for the `common` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();

var setInterval = new CommandLong(
    Param1: Attitude.MsgId,
    Param2: 100000,
    Param3: 0,
    Param4: 0,
    Param5: 0,
    Param6: 0,
    Param7: 0,
    Command: MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    Confirmation: 0);
var intervalFrame = Common.FrameFromGcs(setInterval, 1);
Common.LogFrame("GCS ->", intervalFrame);
var parsedInterval = Common.RoundTripMessage(dialect, setInterval);
if (parsedInterval is CommandLong interval)
{
    Console.WriteLine(
        $"  SET_MESSAGE_INTERVAL msgId={(int)interval.Param1} interval_us={(int)interval.Param2}");
}

var requestOnce = new CommandLong(
    Param1: Attitude.MsgId,
    Param2: 0,
    Param3: 0,
    Param4: 0,
    Param5: 0,
    Param6: 0,
    Param7: 0,
    Command: MavCmd.MAV_CMD_REQUEST_MESSAGE,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    Confirmation: 0);
var onceFrame = Common.FrameFromGcs(requestOnce, 2);
Common.LogFrame("GCS ->", onceFrame);
Common.RoundTripMessage(dialect, requestOnce);

var attitude = new Attitude(
    TimeBootMs: 12345,
    Roll: 0.01f,
    Pitch: -0.02f,
    Yaw: 1.57f,
    Rollspeed: 0,
    Pitchspeed: 0,
    Yawspeed: 0);
var telemetryFrame = Common.FrameFromDrone(attitude, 3);
Common.LogFrame("Drone ->", telemetryFrame);
var parsedAttitude = Common.RoundTripMessage(dialect, attitude);
if (parsedAttitude is Attitude parsed)
{
    Console.WriteLine(
        $"  ATTITUDE roll={parsed.Roll} pitch={parsed.Pitch} yaw={parsed.Yaw}");
}
