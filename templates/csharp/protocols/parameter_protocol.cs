using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Mavlink.Dialects;

namespace Mavlink;

/// <summary>Decoded onboard parameter entry.</summary>
public sealed class ParamEntry
{
    public ParamEntry(string id, decimal value, MavParamType type, int index, int count)
    {
        Id = id;
        Value = value;
        Type = type;
        Index = index;
        Count = count;
    }

    public string Id { get; }

    public decimal Value { get; }

    public MavParamType Type { get; }

    public int Index { get; }

    public int Count { get; }

    public static ParamEntry FromParamValue(ParamValue message) =>
        new(
            ParamCodec.ParamIdToString(message.ParamId),
            ParamCodec.DecodeValue(message.paramValue, message.ParamType),
            message.ParamType,
            message.ParamIndex,
            message.ParamCount);
}

/// <summary>Progress callback for <see cref="ParameterProtocol.FetchAllAsync"/> and <see cref="FetchAllStreamAsync"/>.</summary>
public delegate void ParamProgressCallback(ParamEntry entry, int received, int expected);

/// <summary>Stored parameter value and type on the vehicle.</summary>
public readonly record struct ParamStoreEntry(decimal Value, MavParamType Type);

/// <summary>GCS-side MAVLink parameter protocol client.</summary>
public sealed class ParameterProtocol
{
    private readonly MavlinkSession _session;
    private readonly Dictionary<string, ParamEntry> _cache = new();

    public ParameterProtocol(
        MavlinkSession session,
        byte targetSystem,
        byte targetComponent,
        TimeSpan? idleTimeout = null,
        TimeSpan? requestTimeout = null)
    {
        _session = session;
        TargetSystem = targetSystem;
        TargetComponent = targetComponent;
        IdleTimeout = idleTimeout ?? TimeSpan.FromMilliseconds(500);
        RequestTimeout = requestTimeout ?? TimeSpan.FromSeconds(3);
    }

    public byte TargetSystem { get; }

    public byte TargetComponent { get; }

    public TimeSpan IdleTimeout { get; }

    public TimeSpan RequestTimeout { get; }

    public IReadOnlyDictionary<string, ParamEntry> Cache => _cache;

    public void ClearCache() => _cache.Clear();

    public MavParamType? TypeForName(string name) =>
        _cache.TryGetValue(name, out var entry) ? entry.Type : null;

    public async Task<IReadOnlyList<ParamEntry>> FetchAllAsync(
        ParamProgressCallback? onProgress = null,
        MavlinkCancellationToken? cancel = null)
    {
        var entries = new List<ParamEntry>();
        await foreach (var entry in FetchAllStreamAsync(cancel).ConfigureAwait(false))
        {
            entries.Add(entry);
            onProgress?.Invoke(entry, entries.Count, entry.Count);
        }

        return entries;
    }

    public async IAsyncEnumerable<ParamEntry> FetchAllStreamAsync(
        MavlinkCancellationToken? cancel = null,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        cancel?.ThrowIfCancelled();

        var inbox = Channel.CreateUnbounded<ParamValue>();
        var subscription = _session.ListenMessage<ParamValue>(
            (message, _) => inbox.Writer.TryWrite(message),
            fromSystemId: TargetSystem,
            fromComponentId: TargetComponent);

        try
        {
            await _session.SendAsync(
                new ParamRequestList(TargetSystem: TargetSystem, TargetComponent: TargetComponent),
                cancellationToken).ConfigureAwait(false);

            var expectedCount = -1;
            var seenIndices = new HashSet<int>();
            var retryCounts = new Dictionary<int, int>();
            var isRetrying = false;

            while (true)
            {
                cancel?.ThrowIfCancelled();

                if (!TryTakeNextParam(inbox.Reader, seenIndices, out var paramValue))
                {
                    var waitTimeout = expectedCount == -1 || isRetrying ? RequestTimeout : IdleTimeout;
                    try
                    {
                        paramValue = await WaitForNextParamAsync(
                            inbox.Reader,
                            seenIndices,
                            waitTimeout,
                            cancellationToken,
                            cancel).ConfigureAwait(false);
                    }
                    catch (MavlinkCancelledException)
                    {
                        throw;
                    }
                    catch (MavlinkTimeoutException)
                    {
                        if (expectedCount == -1)
                        {
                            throw;
                        }

                        var missingIndex = FindMissingIndex(seenIndices, expectedCount);
                        if (missingIndex < 0)
                        {
                            break;
                        }

                        var retries = retryCounts.GetValueOrDefault(missingIndex);
                        if (retries >= 3)
                        {
                            throw;
                        }

                        retryCounts[missingIndex] = retries + 1;
                        isRetrying = true;
                        await _session.SendAsync(
                            new ParamRequestRead(
                                ParamIndex: (short)missingIndex,
                                TargetSystem: TargetSystem,
                                TargetComponent: TargetComponent,
                                ParamId: ParamCodec.ParamIdFromString(string.Empty)),
                            cancellationToken).ConfigureAwait(false);
                        continue;
                    }

                    if (paramValue is null)
                    {
                        break;
                    }
                }

                isRetrying = false;

                seenIndices.Add(paramValue.ParamIndex);
                if (expectedCount == -1)
                {
                    expectedCount = paramValue.ParamCount;
                }

                var entry = ParamEntry.FromParamValue(paramValue);
                _cache[entry.Id] = entry;
                yield return entry;

                if (seenIndices.Count >= expectedCount)
                {
                    break;
                }
            }
        }
        finally
        {
            subscription.Cancel();
            inbox.Writer.TryComplete();
        }
    }

    private static bool TryTakeNextParam(
        ChannelReader<ParamValue> reader,
        ISet<int> seenIndices,
        out ParamValue paramValue)
    {
        while (reader.TryRead(out var buffered))
        {
            if (!seenIndices.Contains(buffered.ParamIndex))
            {
                paramValue = buffered;
                return true;
            }
        }

        paramValue = null!;
        return false;
    }

    private static async Task<ParamValue?> WaitForNextParamAsync(
        ChannelReader<ParamValue> reader,
        ISet<int> seenIndices,
        TimeSpan timeout,
        CancellationToken cancellationToken,
        MavlinkCancellationToken? cancel)
    {
        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(timeout);
        if (cancel is not null)
        {
            cancel.OnCancel += () => timeoutCts.Cancel();
        }

        try
        {
            while (await reader.WaitToReadAsync(timeoutCts.Token).ConfigureAwait(false))
            {
                while (reader.TryRead(out var next))
                {
                    if (!seenIndices.Contains(next.ParamIndex))
                    {
                        return next;
                    }
                }
            }

            return null;
        }
        catch (OperationCanceledException) when (cancel?.IsCancelled == true)
        {
            throw new MavlinkCancelledException();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException)
        {
            throw new MavlinkTimeoutException("Timed out waiting for parameter", timeout);
        }
    }

    private static int FindMissingIndex(ISet<int> seenIndices, int expectedCount)
    {
        for (var index = 0; index < expectedCount; index++)
        {
            if (!seenIndices.Contains(index))
            {
                return index;
            }
        }

        return -1;
    }

    public Task<ParamEntry> ReadByNameAsync(string name, MavlinkCancellationToken? cancel = null) =>
        ReadAsync(paramId: name, cancel: cancel);

    public Task<ParamEntry> ReadByIndexAsync(int index, MavlinkCancellationToken? cancel = null) =>
        ReadAsync(paramIndex: index, cancel: cancel);

    public async Task<ParamEntry> ReadAsync(
        string? paramId = null,
        int paramIndex = -1,
        MavlinkCancellationToken? cancel = null)
    {
        if (paramId is null && paramIndex < 0)
        {
            throw new ArgumentException("Either paramId or a non-negative paramIndex is required");
        }

        await _session.SendAsync(
            new ParamRequestRead(
                ParamIndex: paramIndex < 0 ? (short)-1 : (short)paramIndex,
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                ParamId: ParamCodec.ParamIdFromString(paramId ?? string.Empty)),
            CancellationToken.None).ConfigureAwait(false);

        var value = await _session.WaitForMessageTypeAsync<ParamValue>(
            fromSystemId: TargetSystem,
            timeout: RequestTimeout,
            cancel: cancel).ConfigureAwait(false);

        var entry = ParamEntry.FromParamValue(value);
        _cache[entry.Id] = entry;
        return entry;
    }

    public async Task<ParamEntry> WriteAsync(
        string name,
        decimal value,
        MavParamType type,
        MavlinkCancellationToken? cancel = null)
    {
        await _session.SendAsync(
            new ParamSet(
                ParamValue: ParamCodec.EncodeValue(value, type),
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                ParamId: ParamCodec.ParamIdFromString(name),
                ParamType: type),
            CancellationToken.None).ConfigureAwait(false);

        var ack = await _session.WaitForMessageAsync(
            message =>
            {
                if (message is not ParamValue paramValue)
                {
                    return false;
                }

                return ParamCodec.ParamIdToString(paramValue.ParamId) == name;
            },
            fromSystemId: TargetSystem,
            timeout: RequestTimeout,
            cancel: cancel).ConfigureAwait(false);

        var entry = ParamEntry.FromParamValue((ParamValue)ack);
        _cache[entry.Id] = entry;
        return entry;
    }

    public Task<ParamEntry> WriteByNameAsync(
        string name,
        decimal value,
        MavParamType? type = null,
        MavlinkCancellationToken? cancel = null)
    {
        var resolvedType = type ?? TypeForName(name) ?? MavParamType.MAV_PARAM_TYPE_REAL32;
        return WriteAsync(name, value, resolvedType, cancel);
    }
}

/// <summary>Vehicle-side parameter store handler for embedding in autopilot code.</summary>
public sealed class ParameterServer : IAsyncDisposable
{
    private readonly MavlinkSession _session;
    private readonly Dictionary<string, ParamStoreEntry> _values;
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _frameTask;

    public ParameterServer(
        MavlinkSession session,
        IReadOnlyDictionary<string, ParamStoreEntry>? initialValues = null)
    {
        _session = session;
        _values = initialValues is null
            ? new Dictionary<string, ParamStoreEntry>()
            : new Dictionary<string, ParamStoreEntry>(initialValues);
        _frameTask = Task.Run(ProcessFramesAsync);
    }

    public IReadOnlyDictionary<string, ParamStoreEntry> Values => _values;

    public void Set(string name, decimal value, MavParamType type) =>
        _values[name] = new ParamStoreEntry(value, type);

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
            case ParamRequestList list:
                if (list.TargetSystem != _session.SystemId
                    && list.TargetSystem != (byte)MavComponent.MAV_COMP_ID_ALL)
                {
                    return;
                }

                await BroadcastAllAsync().ConfigureAwait(false);
                break;

            case ParamRequestRead read:
                if (read.TargetSystem != _session.SystemId
                    && read.TargetSystem != (byte)MavComponent.MAV_COMP_ID_ALL)
                {
                    return;
                }

                var entry = ResolveRead(read);
                if (entry is not null)
                {
                    await SendValueAsync(entry.Value.Key, entry.Value.Value, IndexOf(entry.Value.Key))
                        .ConfigureAwait(false);
                }

                break;

            case ParamSet set:
                if (set.TargetSystem != _session.SystemId)
                {
                    return;
                }

                var name = ParamCodec.ParamIdToString(set.ParamId);
                _values[name] = new ParamStoreEntry(
                    ParamCodec.DecodeValue(set.ParamValue, set.ParamType),
                    set.ParamType);
                await SendValueAsync(name, _values[name], IndexOf(name)).ConfigureAwait(false);
                break;
        }
    }

    private async Task BroadcastAllAsync()
    {
        var names = _values.Keys.ToList();
        for (var index = 0; index < names.Count; index++)
        {
            await SendValueAsync(names[index], _values[names[index]], index).ConfigureAwait(false);
        }
    }

    private async Task SendValueAsync(string name, ParamStoreEntry entry, int index)
    {
        await _session.SendAsync(
            new ParamValue(
                paramValue: ParamCodec.EncodeValue(entry.Value, entry.Type),
                ParamCount: (ushort)_values.Count,
                ParamIndex: (ushort)index,
                ParamId: ParamCodec.ParamIdFromString(name),
                ParamType: entry.Type),
            CancellationToken.None).ConfigureAwait(false);
    }

    private KeyValuePair<string, ParamStoreEntry>? ResolveRead(ParamRequestRead request)
    {
        if (request.ParamIndex >= 0)
        {
            var names = _values.Keys.ToList();
            if (request.ParamIndex >= names.Count)
            {
                return null;
            }

            var name = names[request.ParamIndex];
            return new KeyValuePair<string, ParamStoreEntry>(name, _values[name]);
        }

        var id = ParamCodec.ParamIdToString(request.ParamId);
        if (!_values.TryGetValue(id, out var entry))
        {
            return null;
        }

        return new KeyValuePair<string, ParamStoreEntry>(id, entry);
    }

    private int IndexOf(string name) => _values.Keys.ToList().IndexOf(name);
}
