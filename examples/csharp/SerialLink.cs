using System.IO.Ports;
using System.Threading.Channels;
using Mavlink;

namespace MavlinkSitlGcs;

/// <summary><see cref="MavlinkLink"/> implementation over a serial/COM port.</summary>
public sealed class SerialMavlinkLink : MavlinkLink
{
    private readonly SerialPort _port;
    private readonly Channel<byte[]> _receive = Channel.CreateUnbounded<byte[]>();
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _readTask;
    private bool _closed;

    private SerialMavlinkLink(SerialPort port)
    {
        _port = port;
        _readTask = Task.Run(ReadLoopAsync);
    }

    /// <summary>Open <paramref name="portName"/> at <paramref name="baudRate"/> (SITL commonly uses 57600 or 115200).</summary>
    public static SerialMavlinkLink Open(string portName, int baudRate = 57600)
    {
        var port = new SerialPort(portName, baudRate, Parity.None, 8, StopBits.One)
        {
            DtrEnable = true,
            RtsEnable = true,
            ReadTimeout = 50,
        };
        port.Open();
        return new SerialMavlinkLink(port);
    }

    public override IAsyncEnumerable<byte[]> Receive => _receive.Reader.ReadAllAsync();

    public override Task SendAsync(byte[] data, CancellationToken cancellationToken = default)
    {
        if (_closed)
        {
            throw new InvalidOperationException("SerialMavlinkLink is closed");
        }

        _port.Write(data, 0, data.Length);
        return Task.CompletedTask;
    }

    public override async Task CloseAsync()
    {
        if (_closed)
        {
            return;
        }

        _closed = true;
        _cts.Cancel();

        if (_port.IsOpen)
        {
            _port.Close();
        }

        _receive.Writer.TryComplete();
        try
        {
            await _readTask.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }

        _cts.Dispose();
        _port.Dispose();
    }

    private async Task ReadLoopAsync()
    {
        var buffer = new byte[4096];

        while (!_closed && _port.IsOpen)
        {
            try
            {
                var bytesAvailable = _port.BytesToRead;
                if (bytesAvailable > 0)
                {
                    var count = _port.Read(buffer, 0, Math.Min(bytesAvailable, buffer.Length));
                    if (count > 0)
                    {
                        var chunk = new byte[count];
                        Array.Copy(buffer, chunk, count);
                        await _receive.Writer.WriteAsync(chunk, _cts.Token).ConfigureAwait(false);
                    }
                }
                else
                {
                    await Task.Delay(10, _cts.Token).ConfigureAwait(false);
                }
            }
            catch (OperationCanceledException) when (_closed || _cts.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex) when (!_closed)
            {
                _receive.Writer.TryComplete(ex);
                break;
            }
        }
    }
}
