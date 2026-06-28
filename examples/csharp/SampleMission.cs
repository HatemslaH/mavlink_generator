using Mavlink;
using Mavlink.Dialects;

namespace MavlinkSitlGcs;

/// <summary>Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).</summary>
public static class SampleMission
{
    public static List<MissionItemInt> Build(byte targetSystem, byte targetComponent) =>
        MissionItems.WithSequentialSeq(
        [
            MissionItems.Waypoint(
                0,
                47.397742,
                8.545594,
                50,
                targetSystem,
                targetComponent),
            MissionItems.Waypoint(
                1,
                47.398000,
                8.546000,
                50,
                targetSystem,
                targetComponent),
            MissionItems.Waypoint(
                2,
                47.398258,
                8.546406,
                50,
                targetSystem,
                targetComponent,
                command: MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH),
        ]);

    public static string Describe(MissionItemInt item)
    {
        var lat = item.X / 1e7;
        var lon = item.Y / 1e7;
        return $"seq={item.Seq} {item.Command} lat={lat:F6} lon={lon:F6} alt={item.Z}m";
    }
}
