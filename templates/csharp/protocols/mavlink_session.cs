using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Mavlink.Dialects;

namespace Mavlink;

/// <summary>Thrown when an expected MAVLink message is not received in time.</summary>
public sealed class MavlinkTimeoutException : Exception
{
    public MavlinkTimeoutException(string message, TimeSpan timeout)
        : base($"{message} (timeout: {timeout})")
    {
        MessageText = message;
        Timeout = timeout;
    }

    public string MessageText { get; }

    public TimeSpan Timeout { get; }
}

/// <summary>
/// Handle returned by <see cref="MavlinkSession.ListenMessage{T}"/>; call <see cref="Cancel"/> to unsubscribe.
/// </summary>
public sealed class MavlinkMessageSubscription
{
    private readonly Action _cancel;
    private bool _active = true;

    internal MavlinkMessageSubscription(Action cancel) => _cancel = cancel;

    public bool IsActive => _active;

    public void Cancel()
    {
        if (!_active)
        {
            return;
        }

        _active = false;
        _cancel();
    }
}

/// <summary>
/// Framing, sequencing, and message dispatch over a <see cref="MavlinkLink"/>.
/// </summary>
public sealed class MavlinkSession : IAsyncDisposable
{
    private const int RecentFrameCapacity = 64;

    private readonly MavlinkDialect _dialect;
    private readonly MavlinkLink _link;
    private readonly MavlinkParser _parser;
    private readonly Channel<MavlinkFrame> _frames = Channel.CreateUnbounded<MavlinkFrame>();
    private readonly List<PendingFrameWait> _pendingWaits = new();
    private readonly List<MavlinkFrame> _recentFrames = new();
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _receiveTask;

    private int _parsedFrameCount;
    private int _sequence;
    private bool _closed;

    public MavlinkSession(
        MavlinkDialect dialect,
        MavlinkLink link,
        byte systemId,
        byte componentId,
        MavlinkVersion version = MavlinkVersion.V2)
    {
        _dialect = dialect;
        _link = link;
        SystemId = systemId;
        ComponentId = componentId;
        Version = version;
        _parser = new MavlinkParser(dialect);
        _receiveTask = Task.Run(ProcessReceiveAsync);
    }

    public MavlinkDialect Dialect => _dialect;

    public byte SystemId { get; }

    public byte ComponentId { get; }

    public MavlinkVersion Version { get; }

    /// <summary>All frames parsed from the link (before filtering).</summary>
    public IAsyncEnumerable<MavlinkFrame> Frames => _frames.Reader.ReadAllAsync();

    /// <summary>Typed message stream filtered by <paramref name="fromSystemId"/> / <paramref name="fromComponentId"/>.</summary>
    public async IAsyncEnumerable<T> OnMessage<T>(
        byte? fromSystemId = null,
        byte? fromComponentId = null,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
        where T : MavlinkMessage
    {
        await foreach (var frame in Frames.WithCancellation(cancellationToken).ConfigureAwait(false))
        {
            if (fromSystemId is not null && frame.SystemId != fromSystemId.Value)
            {
                continue;
            }

            if (fromComponentId is not null && frame.ComponentId != fromComponentId.Value)
            {
                continue;
            }

            if (frame.Message is T message)
            {
                yield return message;
            }
        }
    }

    /// <summary>Message stream filtered by MAVLink message id.</summary>
    public async IAsyncEnumerable<MavlinkMessage> SubscribeMessageId(
        int messageId,
        byte? fromSystemId = null,
        byte? fromComponentId = null,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await foreach (var frame in Frames.WithCancellation(cancellationToken).ConfigureAwait(false))
        {
            if (frame.Message.MavlinkMessageId != messageId)
            {
                continue;
            }

            if (fromSystemId is not null && frame.SystemId != fromSystemId.Value)
            {
                continue;
            }

            if (fromComponentId is not null && frame.ComponentId != fromComponentId.Value)
            {
                continue;
            }

            yield return frame.Message;
        }
    }

    /// <summary>Register a callback for messages of type <typeparamref name="T"/>.</summary>
    public MavlinkMessageSubscription ListenMessage<T>(
        Action<T, MavlinkFrame> onData,
        byte? fromSystemId = null,
        byte? fromComponentId = null)
        where T : MavlinkMessage
    {
        var cts = new CancellationTokenSource();
        var task = Task.Run(async () =>
        {
            await foreach (var frame in Frames.WithCancellation(cts.Token).ConfigureAwait(false))
            {
                if (fromSystemId is not null && frame.SystemId != fromSystemId.Value)
                {
                    continue;
                }

                if (fromComponentId is not null && frame.ComponentId != fromComponentId.Value)
                {
                    continue;
                }

                if (frame.Message is T message)
                {
                    onData(message, frame);
                }
            }
        }, cts.Token);

        return new MavlinkMessageSubscription(() =>
        {
            cts.Cancel();
            cts.Dispose();
            _ = task;
        });
    }

    /// <summary>Send a typed MAVLink message as a framed packet.</summary>
    public async Task SendAsync(MavlinkMessage message, CancellationToken cancellationToken = default)
    {
        if (_closed)
        {
            throw new InvalidOperationException("MavlinkSession is closed");
        }

        var frame = Version == MavlinkVersion.V2
            ? MavlinkFrame.V2((byte)(_sequence++ & 0xFF), SystemId, ComponentId, message)
            : MavlinkFrame.V1((byte)(_sequence++ & 0xFF), SystemId, ComponentId, message);

        await _link.SendAsync(frame.Serialize(), cancellationToken).ConfigureAwait(false);
    }

    /// <summary>Wait for the first frame matching <paramref name="predicate"/>.</summary>
    public Task<MavlinkFrame> WaitForFrameAsync(
        Func<MavlinkFrame, bool> predicate,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        cancel?.ThrowIfCancelled();
        var effectiveTimeout = timeout ?? TimeSpan.FromSeconds(5);
        var tcs = new TaskCompletionSource<MavlinkFrame>(TaskCreationOptions.RunContinuationsAsynchronously);
        var wait = new PendingFrameWait(predicate, tcs, effectiveTimeout, cancel);
        RegisterWait(wait);

        foreach (var frame in _recentFrames.ToList())
        {
            if (!predicate(frame))
            {
                continue;
            }

            _recentFrames.Remove(frame);
            CompleteWait(wait);
            tcs.TrySetResult(frame);
            return tcs.Task;
        }

        return tcs.Task;
    }

    /// <summary>Wait for the first message matching <paramref name="predicate"/>.</summary>
    public async Task<MavlinkMessage> WaitForMessageAsync(
        Func<MavlinkMessage, bool> predicate,
        byte? fromSystemId = null,
        byte? fromComponentId = null,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        var frame = await WaitForFrameAsync(
            frame =>
            {
                if (fromSystemId is not null && frame.SystemId != fromSystemId.Value)
                {
                    return false;
                }

                if (fromComponentId is not null && frame.ComponentId != fromComponentId.Value)
                {
                    return false;
                }

                return predicate(frame.Message);
            },
            timeout,
            cancel).ConfigureAwait(false);

        return frame.Message;
    }

    /// <summary>Wait for the first message of type <typeparamref name="T"/>.</summary>
    public async Task<T> WaitForMessageTypeAsync<T>(
        byte? fromSystemId = null,
        byte? fromComponentId = null,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
        where T : MavlinkMessage
    {
        var message = await WaitForMessageAsync(
            message => message is T,
            fromSystemId,
            fromComponentId,
            timeout,
            cancel).ConfigureAwait(false);

        return (T)message;
    }

    public async ValueTask DisposeAsync()
    {
        await CloseAsync().ConfigureAwait(false);
    }

    public async Task CloseAsync()
    {
        if (_closed)
        {
            return;
        }

        _closed = true;
        _cts.Cancel();

        foreach (var wait in _pendingWaits.ToList())
        {
            CompleteWait(wait);
            wait.Completion.TrySetException(new InvalidOperationException("MavlinkSession is closed"));
        }

        _pendingWaits.Clear();
        _frames.Writer.TryComplete();

        try
        {
            await _receiveTask.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }

        await _link.CloseAsync().ConfigureAwait(false);
        _cts.Dispose();
    }

    private async Task ProcessReceiveAsync()
    {
        try
        {
            await foreach (var chunk in _link.Receive.WithCancellation(_cts.Token).ConfigureAwait(false))
            {
                _parser.Parse(chunk);
                while (_parsedFrameCount < _parser.Frames.Count)
                {
                    OnFrame(_parser.Frames[_parsedFrameCount++]);
                }
            }
        }
        catch (OperationCanceledException) when (_closed || _cts.IsCancellationRequested)
        {
        }
    }

    private void OnFrame(MavlinkFrame frame)
    {
        if (_closed)
        {
            return;
        }

        _frames.Writer.TryWrite(frame);
        _recentFrames.Add(frame);
        if (_recentFrames.Count > RecentFrameCapacity)
        {
            _recentFrames.RemoveAt(0);
        }

        foreach (var wait in _pendingWaits.ToList())
        {
            if (!wait.Predicate(frame))
            {
                continue;
            }

            _recentFrames.Remove(frame);
            CompleteWait(wait);
            wait.Completion.TrySetResult(frame);
            break;
        }
    }

    private void RegisterWait(PendingFrameWait wait)
    {
        _pendingWaits.Add(wait);
        wait.Timer = new Timer(
            _ =>
            {
                if (_pendingWaits.Remove(wait))
                {
                    wait.Completion.TrySetException(
                        new MavlinkTimeoutException("Timed out waiting for frame", wait.Timeout));
                    wait.DisposeRegistration();
                }
            },
            null,
            wait.Timeout,
            Timeout.InfiniteTimeSpan);

        if (wait.Cancel is not null)
        {
            wait.CancelRegistration = wait.Cancel.Register(() =>
            {
                if (_pendingWaits.Remove(wait))
                {
                    wait.Timer?.Dispose();
                    wait.Completion.TrySetException(new MavlinkCancelledException());
                    wait.DisposeRegistration();
                }
            });
        }

        _ = wait.Completion.Task.ContinueWith(
            _ =>
            {
                CompleteWait(wait);
                wait.DisposeRegistration();
            },
            TaskScheduler.Default);
    }

    private static void CompleteWait(PendingFrameWait wait)
    {
        wait.Timer?.Dispose();
        wait.Timer = null;
    }

    private sealed class PendingFrameWait
    {
        public PendingFrameWait(
            Func<MavlinkFrame, bool> predicate,
            TaskCompletionSource<MavlinkFrame> completion,
            TimeSpan timeout,
            MavlinkCancellationToken? cancel)
        {
            Predicate = predicate;
            Completion = completion;
            Timeout = timeout;
            Cancel = cancel;
        }

        public Func<MavlinkFrame, bool> Predicate { get; }

        public TaskCompletionSource<MavlinkFrame> Completion { get; }

        public TimeSpan Timeout { get; }

        public MavlinkCancellationToken? Cancel { get; }

        public Timer? Timer { get; set; }

        public IDisposable? CancelRegistration { get; set; }

        public void DisposeRegistration() => CancelRegistration?.Dispose();
    }
}
