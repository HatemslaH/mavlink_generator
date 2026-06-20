namespace Mavlink;

/// <summary>Thrown when a MAVLink wait or long-running protocol operation is cancelled.</summary>
public sealed class MavlinkCancelledException : Exception
{
    public MavlinkCancelledException(string message = "Operation cancelled")
        : base(message)
    {
    }
}

/// <summary>
/// Cooperative cancellation token for <see cref="MavlinkSession"/> waits and protocol flows.
/// </summary>
public sealed class MavlinkCancellationToken : IDisposable
{
    private readonly object _lock = new();
    private readonly List<Action> _callbacks = new();
    private bool _cancelled;

    public bool IsCancelled
    {
        get
        {
            lock (_lock)
            {
                return _cancelled;
            }
        }
    }

    /// <summary>Fires once when <see cref="Cancel"/> is called.</summary>
    public event Action? OnCancel;

    public void Cancel()
    {
        List<Action>? callbacks;
        lock (_lock)
        {
            if (_cancelled)
            {
                return;
            }

            _cancelled = true;
            callbacks = _callbacks.ToList();
            _callbacks.Clear();
        }

        OnCancel?.Invoke();
        foreach (var callback in callbacks)
        {
            callback();
        }
    }

    public void ThrowIfCancelled()
    {
        if (IsCancelled)
        {
            throw new MavlinkCancelledException();
        }
    }

    internal IDisposable Register(Action callback)
    {
        lock (_lock)
        {
            if (_cancelled)
            {
                callback();
                return new NoopDisposable();
            }

            _callbacks.Add(callback);
            return new Registration(this, callback);
        }
    }

    public void Dispose()
    {
        lock (_lock)
        {
            _callbacks.Clear();
        }
    }

    private void Unregister(Action callback)
    {
        lock (_lock)
        {
            _callbacks.Remove(callback);
        }
    }

    private sealed class Registration(MavlinkCancellationToken owner, Action callback) : IDisposable
    {
        private bool _disposed;

        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
            owner.Unregister(callback);
        }
    }

    private sealed class NoopDisposable : IDisposable
    {
        public void Dispose()
        {
        }
    }
}
