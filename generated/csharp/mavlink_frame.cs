namespace Mavlink;

public sealed class MavlinkFrame
{
    public const byte MavlinkStxV1 = 0xFE;
    public const byte MavlinkStxV2 = 0xFD;

    public MavlinkFrame(
        MavlinkVersion version,
        byte sequence,
        byte systemId,
        byte componentId,
        MavlinkMessage message)
    {
        Version = version;
        Sequence = sequence;
        SystemId = systemId;
        ComponentId = componentId;
        Message = message;
    }

    public MavlinkVersion Version { get; }

    public byte Sequence { get; }

    public byte SystemId { get; }

    public byte ComponentId { get; }

    public MavlinkMessage Message { get; }

    public static MavlinkFrame V1(byte sequence, byte systemId, byte componentId, MavlinkMessage message) =>
        new(MavlinkVersion.V1, sequence, systemId, componentId, message);

    public static MavlinkFrame V2(byte sequence, byte systemId, byte componentId, MavlinkMessage message) =>
        new(MavlinkVersion.V2, sequence, systemId, componentId, message);

    public byte[] Serialize() =>
        Version == MavlinkVersion.V1 ? SerializeV1() : SerializeV2();

    private byte[] SerializeV1()
    {
        var payload = Message.Serialize();
        var payloadLength = (byte)payload.Length;
        var frame = new byte[8 + payloadLength];
        frame[0] = MavlinkStxV1;
        frame[1] = payloadLength;
        frame[2] = Sequence;
        frame[3] = SystemId;
        frame[4] = ComponentId;
        frame[5] = (byte)Message.MavlinkMessageId;

        var crc = new CrcX25();
        crc.Accumulate(payloadLength);
        crc.Accumulate(Sequence);
        crc.Accumulate(SystemId);
        crc.Accumulate(ComponentId);
        crc.Accumulate((byte)Message.MavlinkMessageId);

        for (var i = 0; i < payloadLength; i++)
        {
            frame[6 + i] = payload[i];
            crc.Accumulate(payload[i]);
        }
        crc.Accumulate((byte)Message.MavlinkCrcExtra);

        frame[^2] = (byte)(crc.Crc & 0xFF);
        frame[^1] = (byte)((crc.Crc >> 8) & 0xFF);
        return frame;
    }

    private byte[] SerializeV2()
    {
        const byte incompatibilityFlags = 0;
        const byte compatibilityFlags = 0;
        var payload = TrimTrailingZeros(Message.Serialize());
        var payloadLength = (byte)payload.Length;
        var messageId = Message.MavlinkMessageId;
        var messageIdBytes = new[]
        {
            (byte)(messageId & 0xFF),
            (byte)((messageId >> 8) & 0xFF),
            (byte)((messageId >> 16) & 0xFF),
        };

        var frame = new byte[12 + payloadLength];
        frame[0] = MavlinkStxV2;
        frame[1] = payloadLength;
        frame[2] = incompatibilityFlags;
        frame[3] = compatibilityFlags;
        frame[4] = Sequence;
        frame[5] = SystemId;
        frame[6] = ComponentId;
        frame[7] = messageIdBytes[0];
        frame[8] = messageIdBytes[1];
        frame[9] = messageIdBytes[2];

        var crc = new CrcX25();
        crc.Accumulate(payloadLength);
        crc.Accumulate(incompatibilityFlags);
        crc.Accumulate(compatibilityFlags);
        crc.Accumulate(Sequence);
        crc.Accumulate(SystemId);
        crc.Accumulate(ComponentId);
        foreach (var messageIdByte in messageIdBytes)
        {
            crc.Accumulate(messageIdByte);
        }

        for (var i = 0; i < payloadLength; i++)
        {
            frame[10 + i] = payload[i];
            crc.Accumulate(payload[i]);
        }
        crc.Accumulate((byte)Message.MavlinkCrcExtra);

        frame[^2] = (byte)(crc.Crc & 0xFF);
        frame[^1] = (byte)((crc.Crc >> 8) & 0xFF);
        return frame;
    }

    private static byte[] TrimTrailingZeros(byte[] payload)
    {
        var trimmedLength = payload.Length;
        while (trimmedLength > 0 && payload[trimmedLength - 1] == 0)
        {
            trimmedLength--;
        }

        if (trimmedLength == payload.Length)
        {
            return payload;
        }

        return payload.AsSpan(0, trimmedLength).ToArray();
    }
}
