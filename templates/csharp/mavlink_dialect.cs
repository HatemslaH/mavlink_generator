namespace Mavlink;

public abstract class MavlinkDialect
{
    public abstract int Version { get; }

    public abstract MavlinkMessage? Parse(int messageId, ReadOnlySpan<byte> data);

    /// <summary>Return CRC extra for messageId, or -1 if unsupported.</summary>
    public abstract int CrcExtra(int messageId);
}
