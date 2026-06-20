using System.Threading.Channels;

namespace Mavlink;

/// <summary>Transport-agnostic MAVLink byte stream.</summary>
/// <remarks>
/// Implement for any physical or logical link (USB serial, UDP, TCP, WebSocket,
/// in-memory loopback, etc.). Protocol classes depend only on <see cref="MavlinkLink"/>,
/// not on how bytes are moved.
/// </remarks>
public abstract class MavlinkLink
{
    /// <summary>Send raw MAVLink frame bytes to the remote peer.</summary>
    public abstract Task SendAsync(byte[] data, CancellationToken cancellationToken = default);

    /// <summary>Incoming raw bytes from the remote peer.</summary>
    public abstract IAsyncEnumerable<byte[]> Receive { get; }

    /// <summary>Release link resources. Default implementation is a no-op.</summary>
    public virtual Task CloseAsync() => Task.CompletedTask;
}

/// <summary>
/// In-memory link for tests and virtual examples.
/// </summary>
/// <remarks>
/// Connect two or more endpoints on the same <see cref="VirtualMavlinkBus"/>. Bytes sent by
/// one endpoint are delivered to every other endpoint on the bus.
/// </remarks>
public sealed class VirtualMavlinkBus
{
    private readonly List<VirtualMavlinkEndpoint> _endpoints = new();

    /// <summary>Create a new endpoint on this bus.</summary>
    public MavlinkLink CreateEndpoint() => new VirtualMavlinkEndpoint(this);

    internal void Deliver(byte[] data, VirtualMavlinkEndpoint sender)
    {
        foreach (var endpoint in _endpoints)
        {
            if (!ReferenceEquals(endpoint, sender))
            {
                endpoint.Emit(data);
            }
        }
    }

    internal void Remove(VirtualMavlinkEndpoint endpoint) => _endpoints.Remove(endpoint);

    internal void Register(VirtualMavlinkEndpoint endpoint) => _endpoints.Add(endpoint);

    /// <summary>Close every endpoint on the bus.</summary>
    public async Task CloseAllAsync()
    {
        foreach (var endpoint in _endpoints.ToList())
        {
            await endpoint.CloseAsync().ConfigureAwait(false);
        }
    }
}

internal sealed class VirtualMavlinkEndpoint : MavlinkLink
{
    private readonly VirtualMavlinkBus _bus;
    private readonly Channel<byte[]> _receive = Channel.CreateUnbounded<byte[]>();
    private bool _closed;

    internal VirtualMavlinkEndpoint(VirtualMavlinkBus bus)
    {
        _bus = bus;
        _bus.Register(this);
    }

    public override IAsyncEnumerable<byte[]> Receive => _receive.Reader.ReadAllAsync();

    public override Task SendAsync(byte[] data, CancellationToken cancellationToken = default)
    {
        if (_closed)
        {
            throw new InvalidOperationException("VirtualMavlinkEndpoint is closed");
        }

        _bus.Deliver(data.ToArray(), this);
        return Task.CompletedTask;
    }

    internal void Emit(byte[] data)
    {
        if (!_closed)
        {
            _receive.Writer.TryWrite(data);
        }
    }

    public override async Task CloseAsync()
    {
        if (_closed)
        {
            return;
        }

        _closed = true;
        _receive.Writer.TryComplete();
        _bus.Remove(this);
        await Task.CompletedTask.ConfigureAwait(false);
    }
}
