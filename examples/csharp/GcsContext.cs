using Mavlink;

namespace MavlinkSitlGcs;

/// <summary>Ground control station identity (MAVLink convention).</summary>
public static class GcsIdentity
{
    public const byte SystemId = 255;
    public const byte ComponentId = 190;
}

/// <summary>Shared MAVLink GCS state for the interactive SITL example.</summary>
public sealed class GcsContext
{
    public GcsContext(MavlinkGcs gcs, MavlinkNode vehicle, MavlinkVehicleClient client)
    {
        Gcs = gcs;
        Vehicle = vehicle;
        Client = client;
    }

    public MavlinkGcs Gcs { get; }

    public MavlinkNode Vehicle { get; }

    public MavlinkVehicleClient Client { get; }

    /// <summary>Cancels in-flight parameter/mission operations (type <c>cancel</c> in CLI).</summary>
    public MavlinkCancellationToken? OperationCancel { get; set; }

    public MavlinkSession Session => Gcs.Session;

    public HeartbeatMonitor HeartbeatMonitor => Gcs.HeartbeatMonitor;

    public HeartbeatPublisher HeartbeatPublisher => Gcs.HeartbeatPublisher;

    public ParameterProtocol Parameters => Client.Parameters;

    public MissionProtocol Mission => Client.Mission;

    public CommandProtocol Command => Client.Command;

    public byte TargetSystem => Vehicle.SystemId;

    public byte TargetComponent => Vehicle.ComponentId;
}
