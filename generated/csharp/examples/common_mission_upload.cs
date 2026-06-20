/// <summary>
/// Virtual mission upload for the `common` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();
var missionType = MavMissionType.MAV_MISSION_TYPE_MISSION;

var missionItems = new[]
{
    new MissionItem(
        Param1: 0,
        Param2: 2,
        Param3: 0,
        Param4: 0,
        X: 47.397742f,
        Y: 8.545594f,
        Z: 50,
        Seq: 0,
        Command: MavCmd.MAV_CMD_NAV_WAYPOINT,
        TargetSystem: Common.DroneSystemId,
        TargetComponent: Common.DroneComponentId,
        Frame: MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
        Current: 0,
        Autocontinue: 1,
        MissionType: missionType),
    new MissionItem(
        Param1: 0,
        Param2: 2,
        Param3: 0,
        Param4: 0,
        X: 47.398000f,
        Y: 8.546000f,
        Z: 50,
        Seq: 1,
        Command: MavCmd.MAV_CMD_NAV_WAYPOINT,
        TargetSystem: Common.DroneSystemId,
        TargetComponent: Common.DroneComponentId,
        Frame: MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
        Current: 0,
        Autocontinue: 1,
        MissionType: missionType),
};

var seq = 0;

var count = new MissionCount(
    Count: missionItems.Length,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    MissionType: missionType);
var countFrame = Common.FrameFromGcs(count, 1);
Common.LogFrame("GCS ->", countFrame);
Common.RoundTripMessage(dialect, count);

while (seq < missionItems.Length)
{
    var request = new MissionRequest(
        Seq: (ushort)seq,
        TargetSystem: Common.GcsSystemId,
        TargetComponent: Common.GcsComponentId,
        MissionType: missionType);
    var requestFrame = Common.FrameFromDrone(request, (byte)(seq + 10));
    Common.LogFrame("Drone ->", requestFrame);
    Common.RoundTripMessage(dialect, request);

    var item = missionItems[seq];
    var itemFrame = Common.FrameFromGcs(item, (byte)(seq + 20));
    Common.LogFrame("GCS ->", itemFrame);
    var parsedItem = Common.RoundTripMessage(dialect, item);
    if (parsedItem is MissionItem uploaded)
    {
        Console.WriteLine($"  uploaded seq={uploaded.Seq} cmd={uploaded.Command}");
    }

    seq++;
}

var ack = new MissionAck(
    TargetSystem: Common.GcsSystemId,
    TargetComponent: Common.GcsComponentId,
    Type: MavMissionResult.MAV_MISSION_ACCEPTED,
    MissionType: missionType);
var ackFrame = Common.FrameFromDrone(ack, 99);
Common.LogFrame("Drone ->", ackFrame);
var parsedAck = Common.RoundTripMessage(dialect, ack);
if (parsedAck is MissionAck missionAck)
{
    Console.WriteLine($"Mission upload complete: {missionAck.Type}");
}
