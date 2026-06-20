using Mavlink.Dialects;

namespace Mavlink;

/// <summary>Helpers for building and converting mission plan items.</summary>
public static class MissionItems
{
    public static MissionItemInt Waypoint(
        ushort seq,
        double latitude,
        double longitude,
        float altitude,
        byte targetSystem,
        byte targetComponent,
        MavCmd command = MavCmd.MAV_CMD_NAV_WAYPOINT,
        MavFrame frame = MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
        MavMissionType missionType = MavMissionType.MAV_MISSION_TYPE_MISSION,
        float param1 = 0,
        float param2 = 0,
        float param3 = 0,
        float param4 = 0,
        byte current = 0,
        byte autocontinue = 1) =>
        new(
            Param1: param1,
            Param2: param2,
            Param3: param3,
            Param4: param4,
            X: (int)(latitude * 1e7),
            Y: (int)(longitude * 1e7),
            Z: altitude,
            Seq: seq,
            Command: command,
            TargetSystem: targetSystem,
            TargetComponent: targetComponent,
            Frame: frame,
            Current: current,
            Autocontinue: autocontinue,
            MissionType: missionType);

    public static MissionItem ToLegacyItem(MissionItemInt item) =>
        new(
            Param1: item.Param1,
            Param2: item.Param2,
            Param3: item.Param3,
            Param4: item.Param4,
            X: item.X / 1e7f,
            Y: item.Y / 1e7f,
            Z: item.Z,
            Seq: item.Seq,
            Command: item.Command,
            TargetSystem: item.TargetSystem,
            TargetComponent: item.TargetComponent,
            Frame: item.Frame,
            Current: item.Current,
            Autocontinue: item.Autocontinue,
            MissionType: item.MissionType);

    public static MissionItemInt FromLegacyItem(MissionItem item) =>
        new(
            Param1: item.Param1,
            Param2: item.Param2,
            Param3: item.Param3,
            Param4: item.Param4,
            X: (int)(item.X * 1e7),
            Y: (int)(item.Y * 1e7),
            Z: item.Z,
            Seq: item.Seq,
            Command: item.Command,
            TargetSystem: item.TargetSystem,
            TargetComponent: item.TargetComponent,
            Frame: item.Frame,
            Current: item.Current,
            Autocontinue: item.Autocontinue,
            MissionType: item.MissionType);

    public static List<MissionItemInt> WithSequentialSeq(IReadOnlyList<MissionItemInt> items)
    {
        var result = new List<MissionItemInt>(items.Count);
        for (var index = 0; index < items.Count; index++)
        {
            var item = items[index];
            result.Add(new MissionItemInt(
                Param1: item.Param1,
                Param2: item.Param2,
                Param3: item.Param3,
                Param4: item.Param4,
                X: item.X,
                Y: item.Y,
                Z: item.Z,
                Seq: (ushort)index,
                Command: item.Command,
                TargetSystem: item.TargetSystem,
                TargetComponent: item.TargetComponent,
                Frame: item.Frame,
                Current: item.Current,
                Autocontinue: item.Autocontinue,
                MissionType: item.MissionType));
        }

        return result;
    }
}

public delegate void MissionUploadProgressCallback(int sent, int total, MissionItemInt item);

public delegate void MissionDownloadProgressCallback(int received, int total, MissionItemInt item);

/// <summary>Result of <see cref="MissionProtocol.SetCurrentWithCommandAsync"/>.</summary>
public sealed class MissionSetCurrentResult
{
    public MissionSetCurrentResult(int sequence, CommandAck? commandAck = null)
    {
        Sequence = sequence;
        CommandAck = commandAck;
    }

    public int Sequence { get; }

    public CommandAck? CommandAck { get; }
}

/// <summary>GCS-side MAVLink mission protocol client.</summary>
public sealed class MissionProtocol
{
    private readonly MavlinkSession _session;

    public MissionProtocol(
        MavlinkSession session,
        byte targetSystem,
        byte targetComponent,
        TimeSpan? itemTimeout = null,
        TimeSpan? operationTimeout = null)
    {
        _session = session;
        TargetSystem = targetSystem;
        TargetComponent = targetComponent;
        ItemTimeout = itemTimeout ?? TimeSpan.FromSeconds(3);
        OperationTimeout = operationTimeout ?? TimeSpan.FromSeconds(10);
    }

    public byte TargetSystem { get; }

    public byte TargetComponent { get; }

    public TimeSpan ItemTimeout { get; }

    public TimeSpan OperationTimeout { get; }

    public async Task<MavMissionResult> UploadAsync(
        IReadOnlyList<MissionItemInt> items,
        MavMissionType missionType = MavMissionType.MAV_MISSION_TYPE_MISSION,
        MissionUploadProgressCallback? onProgress = null,
        MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();
        var plan = MissionItems.WithSequentialSeq(items);

        await _session.SendAsync(
            new MissionCount(
                Count: (ushort)plan.Count,
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                MissionType: missionType),
            CancellationToken.None).ConfigureAwait(false);

        foreach (var item in plan)
        {
            cancel?.ThrowIfCancelled();

            var request = await _session.WaitForMessageAsync(
                message => IsItemRequest(message, item.Seq, missionType),
                fromSystemId: TargetSystem,
                timeout: ItemTimeout,
                cancel: cancel).ConfigureAwait(false);

            if (request is MissionRequestInt)
            {
                await _session.SendAsync(item, CancellationToken.None).ConfigureAwait(false);
            }
            else if (request is MissionRequest)
            {
                await _session.SendAsync(MissionItems.ToLegacyItem(item), CancellationToken.None)
                    .ConfigureAwait(false);
            }

            onProgress?.Invoke(item.Seq + 1, plan.Count, item);
        }

        var ack = await _session.WaitForMessageTypeAsync<MissionAck>(
            fromSystemId: TargetSystem,
            timeout: OperationTimeout,
            cancel: cancel).ConfigureAwait(false);

        return ack.Type;
    }

    public async Task<IReadOnlyList<MissionItemInt>> DownloadAsync(
        MavMissionType missionType = MavMissionType.MAV_MISSION_TYPE_MISSION,
        MissionDownloadProgressCallback? onProgress = null,
        MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();

        await _session.SendAsync(
            new MissionRequestList(
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                MissionType: missionType),
            CancellationToken.None).ConfigureAwait(false);

        var countMessage = await _session.WaitForMessageTypeAsync<MissionCount>(
            fromSystemId: TargetSystem,
            timeout: OperationTimeout,
            cancel: cancel).ConfigureAwait(false);

        var items = new List<MissionItemInt>();

        for (ushort seq = 0; seq < countMessage.Count; seq++)
        {
            cancel?.ThrowIfCancelled();

            await _session.SendAsync(
                new MissionRequestInt(
                    Seq: seq,
                    TargetSystem: TargetSystem,
                    TargetComponent: TargetComponent,
                    MissionType: missionType),
                CancellationToken.None).ConfigureAwait(false);

            var itemMessage = await _session.WaitForMessageAsync(
                message =>
                {
                    if (message is MissionItemInt itemInt)
                    {
                        return itemInt.Seq == seq && itemInt.MissionType == missionType;
                    }

                    if (message is MissionItem item)
                    {
                        return item.Seq == seq && item.MissionType == missionType;
                    }

                    return false;
                },
                fromSystemId: TargetSystem,
                timeout: ItemTimeout,
                cancel: cancel).ConfigureAwait(false);

            MissionItemInt itemResult = itemMessage switch
            {
                MissionItemInt itemInt => itemInt,
                MissionItem legacy => MissionItems.FromLegacyItem(legacy),
                _ => throw new InvalidOperationException("Unexpected mission item type"),
            };

            items.Add(itemResult);
            onProgress?.Invoke(items.Count, countMessage.Count, itemResult);
        }

        await _session.SendAsync(
            new MissionAck(
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                Type: MavMissionResult.MAV_MISSION_ACCEPTED,
                MissionType: missionType),
            CancellationToken.None).ConfigureAwait(false);

        return items;
    }

    public async Task<MavMissionResult> ClearAsync(
        MavMissionType missionType = MavMissionType.MAV_MISSION_TYPE_MISSION,
        MavlinkCancellationToken? cancel = null)
    {
        await _session.SendAsync(
            new MissionClearAll(
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                MissionType: missionType),
            CancellationToken.None).ConfigureAwait(false);

        var ack = await _session.WaitForMessageTypeAsync<MissionAck>(
            fromSystemId: TargetSystem,
            timeout: OperationTimeout,
            cancel: cancel).ConfigureAwait(false);

        return ack.Type;
    }

    public async Task SetCurrentAsync(int seq, MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();
        await _session.SendAsync(
            new MissionSetCurrent(
                Seq: (ushort)seq,
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent),
            CancellationToken.None).ConfigureAwait(false);
    }

    public async Task<MissionSetCurrentResult> SetCurrentWithCommandAsync(
        int seq,
        CommandProtocol? command = null,
        bool alsoSendCommand = true,
        bool resetMission = false,
        MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();
        await SetCurrentAsync(seq, cancel).ConfigureAwait(false);

        CommandAck? ack = null;
        if (alsoSendCommand && command is not null)
        {
            ack = await command.SetMissionCurrentAsync(seq, resetMission, cancel: cancel)
                .ConfigureAwait(false);
        }

        return new MissionSetCurrentResult(seq, ack);
    }

    private static bool IsItemRequest(MavlinkMessage message, ushort seq, MavMissionType missionType) =>
        message switch
        {
            MissionRequestInt requestInt => requestInt.Seq == seq && requestInt.MissionType == missionType,
            MissionRequest request => request.Seq == seq && request.MissionType == missionType,
            _ => false,
        };
}

/// <summary>Vehicle-side mission protocol handler for embedding in autopilot code.</summary>
public sealed class MissionServer : IAsyncDisposable
{
    private readonly MavlinkSession _session;
    private readonly MavMissionType _missionType;
    private readonly List<MissionItemInt> _items;
    private readonly Dictionary<int, MissionItemInt> _incoming = new();
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _frameTask;
    private int? _incomingCount;

    public MissionServer(
        MavlinkSession session,
        IReadOnlyList<MissionItemInt>? initialMission = null,
        MavMissionType missionType = MavMissionType.MAV_MISSION_TYPE_MISSION)
    {
        _session = session;
        _missionType = missionType;
        _items = initialMission is null
            ? new List<MissionItemInt>()
            : new List<MissionItemInt>(initialMission);
        _frameTask = Task.Run(ProcessFramesAsync);
    }

    public IReadOnlyList<MissionItemInt> Items => _items;

    public void ReplaceMission(IReadOnlyList<MissionItemInt> items)
    {
        _items.Clear();
        _items.AddRange(MissionItems.WithSequentialSeq(items));
        _incoming.Clear();
        _incomingCount = null;
    }

    public async ValueTask DisposeAsync() => await CloseAsync().ConfigureAwait(false);

    public async Task CloseAsync()
    {
        _cts.Cancel();
        try
        {
            await _frameTask.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }

        _cts.Dispose();
    }

    private async Task ProcessFramesAsync()
    {
        await foreach (var frame in _session.Frames.WithCancellation(_cts.Token).ConfigureAwait(false))
        {
            await OnFrameAsync(frame).ConfigureAwait(false);
        }
    }

    private async Task OnFrameAsync(MavlinkFrame frame)
    {
        switch (frame.Message)
        {
            case MissionCount count when TargetsUs(count.TargetSystem, count.TargetComponent):
                if (count.MissionType != _missionType)
                {
                    return;
                }

                _incomingCount = count.Count;
                _incoming.Clear();
                if (count.Count > 0)
                {
                    await RequestUploadItemAsync(frame, 0).ConfigureAwait(false);
                }
                else
                {
                    await SendUploadAckAsync(frame).ConfigureAwait(false);
                }

                break;

            case MissionItemInt itemInt when TargetsUs(itemInt.TargetSystem, itemInt.TargetComponent):
                if (itemInt.MissionType != _missionType)
                {
                    return;
                }

                await StoreIncomingItemAsync(frame, itemInt).ConfigureAwait(false);
                break;

            case MissionItem item when TargetsUs(item.TargetSystem, item.TargetComponent):
                if (item.MissionType != _missionType)
                {
                    return;
                }

                await StoreIncomingItemAsync(frame, MissionItems.FromLegacyItem(item)).ConfigureAwait(false);
                break;

            case MissionRequestInt requestInt when TargetsUs(requestInt.TargetSystem, requestInt.TargetComponent):
                await SendRequestedItemAsync(frame, requestInt.Seq).ConfigureAwait(false);
                break;

            case MissionRequest request when TargetsUs(request.TargetSystem, request.TargetComponent):
                await SendRequestedItemAsync(frame, request.Seq).ConfigureAwait(false);
                break;

            case MissionRequestList list when TargetsUs(list.TargetSystem, list.TargetComponent):
                if (list.MissionType != _missionType)
                {
                    return;
                }

                await _session.SendAsync(
                    new MissionCount(
                        Count: (ushort)_items.Count,
                        TargetSystem: frame.SystemId,
                        TargetComponent: frame.ComponentId,
                        MissionType: _missionType),
                    CancellationToken.None).ConfigureAwait(false);
                break;

            case MissionClearAll clear when TargetsUs(clear.TargetSystem, clear.TargetComponent):
                if (clear.MissionType != _missionType)
                {
                    return;
                }

                _items.Clear();
                _incoming.Clear();
                _incomingCount = null;
                await _session.SendAsync(
                    new MissionAck(
                        TargetSystem: frame.SystemId,
                        TargetComponent: frame.ComponentId,
                        Type: MavMissionResult.MAV_MISSION_ACCEPTED,
                        MissionType: _missionType),
                    CancellationToken.None).ConfigureAwait(false);
                break;
        }
    }

    private async Task StoreIncomingItemAsync(MavlinkFrame frame, MissionItemInt item)
    {
        _incoming[item.Seq] = item;
        var expected = _incomingCount;
        if (expected is null)
        {
            return;
        }

        if (_incoming.Count < expected)
        {
            await RequestUploadItemAsync(frame, (ushort)(item.Seq + 1)).ConfigureAwait(false);
            return;
        }

        _items.Clear();
        for (var index = 0; index < expected; index++)
        {
            _items.Add(_incoming[index]);
        }

        _incoming.Clear();
        _incomingCount = null;
        await SendUploadAckAsync(frame).ConfigureAwait(false);
    }

    private Task RequestUploadItemAsync(MavlinkFrame requestFrame, ushort seq) =>
        _session.SendAsync(
            new MissionRequestInt(
                Seq: seq,
                TargetSystem: requestFrame.SystemId,
                TargetComponent: requestFrame.ComponentId,
                MissionType: _missionType),
            CancellationToken.None);

    private Task SendUploadAckAsync(MavlinkFrame requestFrame) =>
        _session.SendAsync(
            new MissionAck(
                TargetSystem: requestFrame.SystemId,
                TargetComponent: requestFrame.ComponentId,
                Type: MavMissionResult.MAV_MISSION_ACCEPTED,
                MissionType: _missionType),
            CancellationToken.None);

    private async Task SendRequestedItemAsync(MavlinkFrame requestFrame, ushort seq)
    {
        if (seq >= _items.Count)
        {
            await _session.SendAsync(
                new MissionAck(
                    TargetSystem: requestFrame.SystemId,
                    TargetComponent: requestFrame.ComponentId,
                    Type: MavMissionResult.MAV_MISSION_INVALID_SEQUENCE,
                    MissionType: _missionType),
                CancellationToken.None).ConfigureAwait(false);
            return;
        }

        await _session.SendAsync(_items[seq], CancellationToken.None).ConfigureAwait(false);
    }

    private bool TargetsUs(byte targetSystem, byte targetComponent) =>
        MatchesTarget(targetSystem, targetComponent);

    private bool MatchesTarget(byte targetSystem, byte targetComponent)
    {
        if (targetSystem != _session.SystemId && targetSystem != 0)
        {
            return false;
        }

        if (targetComponent != _session.ComponentId
            && targetComponent != (byte)MavComponent.MAV_COMP_ID_ALL)
        {
            return false;
        }

        return true;
    }
}
