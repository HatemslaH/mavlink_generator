using Mavlink.Dialects;

namespace Mavlink;

/// <summary>
/// Protocol clients bound to a single remote MAVLink vehicle.
/// </summary>
public sealed class MavlinkVehicleClient
{
    public MavlinkVehicleClient(
        MavlinkSession session,
        MavlinkNode vehicle,
        TimeSpan? parameterRequestTimeout = null,
        TimeSpan? parameterIdleTimeout = null,
        TimeSpan? missionItemTimeout = null,
        TimeSpan? missionOperationTimeout = null,
        TimeSpan? commandTimeout = null)
    {
        Session = session;
        Vehicle = vehicle;
        Parameters = new ParameterProtocol(
            session,
            vehicle.SystemId,
            vehicle.ComponentId,
            idleTimeout: parameterIdleTimeout ?? TimeSpan.FromSeconds(2),
            requestTimeout: parameterRequestTimeout ?? TimeSpan.FromSeconds(10));
        Mission = new MissionProtocol(
            session,
            vehicle.SystemId,
            vehicle.ComponentId,
            itemTimeout: missionItemTimeout ?? TimeSpan.FromSeconds(10),
            operationTimeout: missionOperationTimeout ?? TimeSpan.FromSeconds(30));
        Command = new CommandProtocol(
            session,
            vehicle.SystemId,
            vehicle.ComponentId,
            defaultTimeout: commandTimeout ?? TimeSpan.FromSeconds(10));
    }

    public MavlinkSession Session { get; }

    public MavlinkNode Vehicle { get; }

    public ParameterProtocol Parameters { get; }

    public MissionProtocol Mission { get; }

    public CommandProtocol Command { get; }

    public byte TargetSystem => Vehicle.SystemId;

    public byte TargetComponent => Vehicle.ComponentId;
}

/// <summary>
/// Ground control station bootstrap: session, heartbeat publisher, and monitor.
/// </summary>
public sealed class MavlinkGcs : IAsyncDisposable
{
    public MavlinkGcs(
        MavlinkSession session,
        HeartbeatPublisher heartbeatPublisher,
        HeartbeatMonitor heartbeatMonitor)
    {
        Session = session;
        HeartbeatPublisher = heartbeatPublisher;
        HeartbeatMonitor = heartbeatMonitor;
    }

    public MavlinkSession Session { get; }

    public HeartbeatPublisher HeartbeatPublisher { get; }

    public HeartbeatMonitor HeartbeatMonitor { get; }

    public void Start()
    {
        HeartbeatMonitor.Start();
        HeartbeatPublisher.Start();
    }

    public async Task StopHeartbeatsAsync()
    {
        HeartbeatPublisher.Stop();
        await HeartbeatMonitor.StopAsync().ConfigureAwait(false);
    }

    public async Task<MavlinkVehicleClient> WaitForVehicleAsync(
        IReadOnlySet<byte>? excludeSystemIds = null,
        TimeSpan? timeout = null)
    {
        var node = await HeartbeatMonitor.WaitForVehicleAsync(excludeSystemIds, timeout)
            .ConfigureAwait(false);
        return VehicleClient(node);
    }

    public MavlinkVehicleClient VehicleClient(MavlinkNode vehicle) =>
        new(Session, vehicle);

    public static MavlinkGcs Connect(
        MavlinkDialect dialect,
        MavlinkLink link,
        byte systemId = 255,
        byte componentId = 190,
        TimeSpan? heartbeatInterval = null,
        TimeSpan? heartbeatTimeout = null)
    {
        var session = new MavlinkSession(dialect, link, systemId, componentId);
        return new MavlinkGcs(
            session,
            new HeartbeatPublisher(
                session,
                HeartbeatTemplates.Gcs(dialect.Version),
                heartbeatInterval ?? TimeSpan.FromSeconds(1)),
            new HeartbeatMonitor(session, heartbeatTimeout ?? TimeSpan.FromSeconds(3)));
    }

    public async ValueTask DisposeAsync() => await CloseAsync().ConfigureAwait(false);

    public async Task CloseAsync()
    {
        await StopHeartbeatsAsync().ConfigureAwait(false);
        await Session.CloseAsync().ConfigureAwait(false);
    }
}
