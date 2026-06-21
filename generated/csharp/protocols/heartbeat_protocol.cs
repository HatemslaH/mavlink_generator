using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Mavlink.Dialects;

namespace Mavlink;

/// <summary>MAVLink node identity (system + component).</summary>
public readonly record struct MavlinkNode(byte SystemId, byte ComponentId)
{
    public override string ToString() => $"MavlinkNode({SystemId}:{ComponentId})";
}

/// <summary>Last known heartbeat state for a remote node.</summary>
public sealed class TrackedHeartbeat
{
    public TrackedHeartbeat(MavlinkNode node, Heartbeat heartbeat, DateTimeOffset receivedAt, bool online)
    {
        Node = node;
        Heartbeat = heartbeat;
        ReceivedAt = receivedAt;
        Online = online;
    }

    public MavlinkNode Node { get; }

    public Heartbeat Heartbeat { get; }

    public DateTimeOffset ReceivedAt { get; }

    public bool Online { get; }

    public TimeSpan Age => DateTimeOffset.UtcNow - ReceivedAt;
}

/// <summary>
/// Tracks remote HEARTBEAT messages and reports connect / disconnect events.
/// </summary>
public sealed class HeartbeatMonitor : IAsyncDisposable
{
    private readonly MavlinkSession _session;
    private readonly TimeSpan _timeout;
    private readonly HashSet<MavlinkNode>? _watch;
    private readonly byte? _watchSystemId;
    private readonly Dictionary<MavlinkNode, TrackedHeartbeat> _states = new();
    private readonly Dictionary<MavlinkNode, bool> _online = new();
    private readonly Channel<TrackedHeartbeat> _heartbeatEvents = Channel.CreateUnbounded<TrackedHeartbeat>();
    private readonly Channel<MavlinkNode> _connectedEvents = Channel.CreateUnbounded<MavlinkNode>();
    private readonly Channel<MavlinkNode> _disconnectedEvents = Channel.CreateUnbounded<MavlinkNode>();

    private CancellationTokenSource? _cts;
    private Task? _frameTask;
    private PeriodicTimer? _watchdogTimer;
    private bool _running;

    public HeartbeatMonitor(
        MavlinkSession session,
        TimeSpan? timeout = null,
        IReadOnlySet<MavlinkNode>? watch = null,
        byte? watchSystemId = null)
    {
        _session = session;
        _timeout = timeout ?? TimeSpan.FromSeconds(5);
        _watch = watch is null ? null : new HashSet<MavlinkNode>(watch);
        _watchSystemId = watchSystemId;
    }

    /// <summary>Emitted on every received (or recovered) heartbeat update.</summary>
    public IAsyncEnumerable<TrackedHeartbeat> OnHeartbeat => _heartbeatEvents.Reader.ReadAllAsync();

    /// <summary>Emitted when a watched node comes online (first heartbeat or recovery).</summary>
    public IAsyncEnumerable<MavlinkNode> OnConnected => _connectedEvents.Reader.ReadAllAsync();

    /// <summary>Emitted when a watched node times out without heartbeats.</summary>
    public IAsyncEnumerable<MavlinkNode> OnDisconnected => _disconnectedEvents.Reader.ReadAllAsync();

    /// <summary>Start monitoring. Safe to call only once; use <see cref="StopAsync"/> before restarting.</summary>
    public void Start()
    {
        if (_running)
        {
            return;
        }

        _running = true;
        _cts = new CancellationTokenSource();
        _frameTask = Task.Run(ProcessFramesAsync);
        _watchdogTimer = new PeriodicTimer(TimeSpan.FromMilliseconds(_timeout.TotalMilliseconds / 3));
        _ = Task.Run(WatchdogLoopAsync);
    }

    /// <summary>Stop monitoring and release timers/subscriptions.</summary>
    public async Task StopAsync()
    {
        if (!_running)
        {
            return;
        }

        _running = false;
        _cts?.Cancel();
        _watchdogTimer?.Dispose();
        _watchdogTimer = null;

        if (_frameTask is not null)
        {
            try
            {
                await _frameTask.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
            }
        }

        _cts?.Dispose();
        _cts = null;
        _frameTask = null;
    }

    public TrackedHeartbeat? StateFor(MavlinkNode node) =>
        _states.TryGetValue(node, out var state) ? state : null;

    public TrackedHeartbeat? StateForIds(byte systemId, byte componentId) =>
        StateFor(new MavlinkNode(systemId, componentId));

    public bool IsOnline(MavlinkNode node) => _online.TryGetValue(node, out var online) && online;

    public bool IsOnlineIds(byte systemId, byte componentId) => IsOnline(new MavlinkNode(systemId, componentId));

    public IEnumerable<MavlinkNode> OnlineNodes
    {
        get
        {
            foreach (var entry in _online)
            {
                if (entry.Value)
                {
                    yield return entry.Key;
                }
            }
        }
    }

    /// <summary>
    /// Wait until the first online vehicle heartbeat is observed.
    /// </summary>
    public async Task<MavlinkNode> WaitForVehicleAsync(
        IReadOnlySet<byte>? excludeSystemIds = null,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();
        var effectiveTimeout = timeout ?? TimeSpan.FromSeconds(60);

        foreach (var node in OnlineNodes)
        {
            if (excludeSystemIds is null || !excludeSystemIds.Contains(node.SystemId))
            {
                return node;
            }
        }

        using var cts = new CancellationTokenSource();
        cts.CancelAfter(effectiveTimeout);
        if (cancel is not null)
        {
            cancel.OnCancel += () => cts.Cancel();
        }

        try
        {
            await foreach (var node in OnConnected.WithCancellation(cts.Token).ConfigureAwait(false))
            {
                if (excludeSystemIds is null || !excludeSystemIds.Contains(node.SystemId))
                {
                    return node;
                }
            }

            throw new MavlinkTimeoutException("Timed out waiting for vehicle heartbeat", effectiveTimeout);
        }
        catch (OperationCanceledException) when (cancel?.IsCancelled == true)
        {
            throw new MavlinkCancelledException();
        }
        catch (OperationCanceledException)
        {
            throw new MavlinkTimeoutException("Timed out waiting for vehicle heartbeat", effectiveTimeout);
        }
    }

    public async ValueTask DisposeAsync() => await StopAsync().ConfigureAwait(false);

    private async Task ProcessFramesAsync()
    {
        var token = _cts?.Token ?? CancellationToken.None;
        await foreach (var frame in _session.Frames.WithCancellation(token).ConfigureAwait(false))
        {
            if (frame.Message is not Heartbeat heartbeat)
            {
                continue;
            }

            var node = new MavlinkNode(frame.SystemId, frame.ComponentId);
            if (!ShouldWatch(node))
            {
                continue;
            }

            var wasOnline = IsOnline(node);
            var now = DateTimeOffset.UtcNow;
            var tracked = new TrackedHeartbeat(node, heartbeat, now, true);
            _states[node] = tracked;
            _online[node] = true;
            _heartbeatEvents.Writer.TryWrite(tracked);

            if (!wasOnline)
            {
                _connectedEvents.Writer.TryWrite(node);
            }
        }
    }

    private async Task WatchdogLoopAsync()
    {
        if (_watchdogTimer is null || _cts is null)
        {
            return;
        }

        try
        {
            while (await _watchdogTimer.WaitForNextTickAsync(_cts.Token).ConfigureAwait(false))
            {
                CheckTimeouts();
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private void CheckTimeouts()
    {
        var now = DateTimeOffset.UtcNow;
        foreach (var node in _states.Keys.ToList())
        {
            if (!_states.TryGetValue(node, out var state))
            {
                continue;
            }

            var timedOut = now - state.ReceivedAt > _timeout;
            var wasOnline = IsOnline(node);
            if (timedOut && wasOnline)
            {
                _online[node] = false;
                _disconnectedEvents.Writer.TryWrite(node);
                _heartbeatEvents.Writer.TryWrite(
                    new TrackedHeartbeat(node, state.Heartbeat, state.ReceivedAt, false));
            }
        }
    }

    private bool ShouldWatch(MavlinkNode node)
    {
        if (_watch is not null)
        {
            return _watch.Contains(node);
        }

        if (_watchSystemId is not null)
        {
            return node.SystemId == _watchSystemId.Value;
        }

        return true;
    }
}

/// <summary>Periodically sends HEARTBEAT on a <see cref="MavlinkSession"/>.</summary>
public sealed class HeartbeatPublisher : IAsyncDisposable
{
    private readonly MavlinkSession _session;
    private readonly TimeSpan _interval;
    private Heartbeat _heartbeat;
    private PeriodicTimer? _timer;
    private CancellationTokenSource? _cts;
    private bool _running;

    public HeartbeatPublisher(MavlinkSession session, Heartbeat heartbeat, TimeSpan? interval = null)
    {
        _session = session;
        _heartbeat = heartbeat;
        _interval = interval ?? TimeSpan.FromSeconds(1);
    }

    public Heartbeat Heartbeat => _heartbeat;

    public void UpdateHeartbeat(Heartbeat heartbeat) => _heartbeat = heartbeat;

    public void MutateHeartbeat(Func<Heartbeat, Heartbeat> transform) =>
        _heartbeat = transform(_heartbeat);

    public void Start()
    {
        if (_running)
        {
            return;
        }

        _running = true;
        _cts = new CancellationTokenSource();
        _timer = new PeriodicTimer(_interval);
        _ = Task.Run(RunAsync);
        _ = SendOnceAsync();
    }

    public void Stop()
    {
        _running = false;
        _cts?.Cancel();
        _timer?.Dispose();
        _timer = null;
        _cts?.Dispose();
        _cts = null;
    }

    public Task SendOnceAsync() => _session.SendAsync(_heartbeat);

    public ValueTask DisposeAsync()
    {
        Stop();
        return ValueTask.CompletedTask;
    }

    private async Task RunAsync()
    {
        if (_timer is null || _cts is null)
        {
            return;
        }

        try
        {
            while (await _timer.WaitForNextTickAsync(_cts.Token).ConfigureAwait(false))
            {
                await SendOnceAsync().ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }
}

/// <summary>Convenience factories for common HEARTBEAT payloads.</summary>
public static class HeartbeatTemplates
{
    public static Heartbeat Gcs(int mavlinkVersion) =>
        new(
            CustomMode: 0,
            Type: MavType.MAV_TYPE_GCS,
            Autopilot: MavAutopilot.MAV_AUTOPILOT_INVALID,
            BaseMode: 0,
            SystemStatus: MavState.MAV_STATE_ACTIVE,
            MavlinkVersion: (byte)mavlinkVersion);

    public static Heartbeat Autopilot(
        int mavlinkVersion,
        MavType type = MavType.MAV_TYPE_QUADROTOR,
        MavAutopilot autopilot = MavAutopilot.MAV_AUTOPILOT_PX4,
        MavState systemStatus = MavState.MAV_STATE_ACTIVE,
        uint customMode = 0,
        byte baseMode = 0) =>
        new(
            CustomMode: customMode,
            Type: type,
            Autopilot: autopilot,
            BaseMode: baseMode,
            SystemStatus: systemStatus,
            MavlinkVersion: (byte)mavlinkVersion);

    public static Heartbeat OnboardApi(int mavlinkVersion) =>
        new(
            CustomMode: 0,
            Type: MavType.MAV_TYPE_ONBOARD_CONTROLLER,
            Autopilot: MavAutopilot.MAV_AUTOPILOT_INVALID,
            BaseMode: 0,
            SystemStatus: MavState.MAV_STATE_ACTIVE,
            MavlinkVersion: (byte)mavlinkVersion);
}
