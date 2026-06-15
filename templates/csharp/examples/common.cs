using Mavlink;
using Mavlink.Dialects;

/// <summary>Shared helpers for MAVLink C# examples.</summary>
public static class Common
{
    public const byte GcsSystemId = 255;
    public const byte GcsComponentId = 190;

    public const byte DroneSystemId = 1;
    public const byte DroneComponentId = 1;

    public static MavlinkFrame FrameFromGcs(MavlinkMessage message, byte sequence = 0) =>
        MavlinkFrame.V2(sequence, GcsSystemId, GcsComponentId, message);

    public static MavlinkFrame FrameFromDrone(MavlinkMessage message, byte sequence = 0) =>
        MavlinkFrame.V2(sequence, DroneSystemId, DroneComponentId, message);

    public static byte[] ParamIdFromString(string name)
    {
        var paramId = new byte[16];
        var bytes = System.Text.Encoding.ASCII.GetBytes(name);
        Array.Copy(bytes, 0, paramId, 0, Math.Min(bytes.Length, 16));
        return paramId;
    }

    public static string ParamIdToString(byte[] paramId)
    {
        var end = Array.FindIndex(paramId, value => value == 0);
        if (end < 0)
        {
            end = paramId.Length;
        }

        return System.Text.Encoding.ASCII.GetString(paramId, 0, end);
    }

    public static void LogFrame(string direction, MavlinkFrame frame)
    {
        Console.WriteLine(
            $"{direction} msgId={frame.Message.MavlinkMessageId} " +
            $"sys={frame.SystemId} comp={frame.ComponentId}");
    }

    public static MavlinkMessage? RoundTripMessage(MavlinkDialect dialect, MavlinkMessage message) =>
        dialect.Parse(message.MavlinkMessageId, message.Serialize());
}
