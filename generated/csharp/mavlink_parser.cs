namespace Mavlink;

public sealed class MavlinkParser
{
    private const int MavlinkMaximumPayloadSize = 255;
    private const byte MavlinkIflagSigned = 0x01;
    private const int MavlinkSignatureLength = 13;

    private readonly MavlinkDialect _dialect;
    private readonly Action<int>? _onSignedPacketDropped;
    private readonly List<MavlinkFrame> _frames = new();
    private ParserState _state = ParserState.Init;

    private MavlinkVersion _version = MavlinkVersion.V1;
    private int _payloadLength = -1;
    private int _incompatibilityFlags = -1;
    private int _compatibilityFlags = -1;
    private int _sequence = -1;
    private int _systemId = -1;
    private int _componentId = -1;
    private int _messageIdLow = -1;
    private int _messageIdMiddle = -1;
    private int _messageIdHigh = -1;
    private int _messageId = -1;
    private readonly byte[] _payload = new byte[MavlinkMaximumPayloadSize];
    private int _payloadCursor = -1;
    private int _crcLowByte = -1;
    private int _crcHighByte = -1;
    private int _signatureBytesRemaining;

    public MavlinkParser(MavlinkDialect dialect, Action<int>? onSignedPacketDropped = null)
    {
        _dialect = dialect;
        _onSignedPacketDropped = onSignedPacketDropped;
        ResetContext();
    }

    public IReadOnlyList<MavlinkFrame> Frames => _frames;

    public void Parse(ReadOnlySpan<byte> data)
    {
        foreach (var value in data)
        {
            ParseByte(value);
        }
    }

    private void ResetContext()
    {
        _version = MavlinkVersion.V1;
        _payloadLength = -1;
        _incompatibilityFlags = -1;
        _compatibilityFlags = -1;
        _sequence = -1;
        _systemId = -1;
        _componentId = -1;
        _messageIdLow = -1;
        _messageIdMiddle = -1;
        _messageIdHigh = -1;
        _messageId = -1;
        Array.Clear(_payload, 0, _payload.Length);
        _payloadCursor = -1;
        _crcLowByte = -1;
        _crcHighByte = -1;
        _signatureBytesRemaining = 0;
    }

    private bool CheckCrc()
    {
        int[] header;
        if (_version == MavlinkVersion.V1)
        {
            header =
            [
                _payloadLength,
                _sequence,
                _systemId,
                _componentId,
                _messageId,
            ];
        }
        else
        {
            header =
            [
                _payloadLength,
                _incompatibilityFlags,
                _compatibilityFlags,
                _sequence,
                _systemId,
                _componentId,
                _messageIdLow,
                _messageIdMiddle,
                _messageIdHigh,
            ];
        }

        var crc = new CrcX25();
        foreach (var value in header)
        {
            crc.Accumulate((byte)value);
        }

        for (var i = 0; i < _payloadLength; i++)
        {
            crc.Accumulate(_payload[i]);
        }

        var crcExt = _dialect.CrcExtra(_messageId);
        if (crcExt == -1)
        {
            return false;
        }

        crc.Accumulate((byte)crcExt);
        return crc.Crc == ((_crcHighByte << 8) ^ _crcLowByte);
    }

    private void ParseByte(byte value)
    {
        switch (_state)
        {
            case ParserState.Init:
                if (value == MavlinkFrame.MavlinkStxV1)
                {
                    _version = MavlinkVersion.V1;
                    _state = ParserState.WaitPayloadLength;
                }
                else if (value == MavlinkFrame.MavlinkStxV2)
                {
                    _version = MavlinkVersion.V2;
                    _state = ParserState.WaitPayloadLength;
                }
                return;

            case ParserState.WaitPayloadLength:
                _payloadLength = value;
                _state = _version == MavlinkVersion.V1
                    ? ParserState.WaitPacketSequence
                    : ParserState.WaitIncompatibilityFlags;
                return;

            case ParserState.WaitIncompatibilityFlags:
                _incompatibilityFlags = value;
                _state = ParserState.WaitCompatibilityFlags;
                return;

            case ParserState.WaitCompatibilityFlags:
                _compatibilityFlags = value;
                _state = ParserState.WaitPacketSequence;
                return;

            case ParserState.WaitPacketSequence:
                _sequence = value;
                _state = ParserState.WaitSystemId;
                return;

            case ParserState.WaitSystemId:
                _systemId = value;
                _state = ParserState.WaitComponentId;
                return;

            case ParserState.WaitComponentId:
                _componentId = value;
                _state = _version == MavlinkVersion.V1
                    ? ParserState.WaitMessageIdHigh
                    : ParserState.WaitMessageIdLow;
                return;

            case ParserState.WaitMessageIdLow:
                _messageIdLow = value;
                _state = ParserState.WaitMessageIdMiddle;
                return;

            case ParserState.WaitMessageIdMiddle:
                _messageIdMiddle = value;
                _state = ParserState.WaitMessageIdHigh;
                return;

            case ParserState.WaitMessageIdHigh:
                if (_version == MavlinkVersion.V1)
                {
                    _messageId = value;
                }
                else
                {
                    _messageIdHigh = value;
                    _messageId = (_messageIdHigh << 16) ^ (_messageIdMiddle << 8) ^ _messageIdLow;
                }

                if (_payloadLength == 0)
                {
                    _state = ParserState.WaitCrcLowByte;
                }
                else
                {
                    _state = ParserState.WaitPayloadEnd;
                    _payloadCursor = 0;
                }
                return;

            case ParserState.WaitPayloadEnd:
                if (_payloadCursor < _payloadLength)
                {
                    _payload[_payloadCursor] = value;
                    _payloadCursor++;
                }

                if (_payloadCursor == _payloadLength)
                {
                    _state = ParserState.WaitCrcLowByte;
                }
                return;

            case ParserState.WaitCrcLowByte:
                _crcLowByte = value;
                _state = ParserState.WaitCrcHighByte;
                return;

            case ParserState.WaitCrcHighByte:
                _crcHighByte = value;
                if (_version == MavlinkVersion.V2
                    && (_incompatibilityFlags & MavlinkIflagSigned) != 0)
                {
                    _onSignedPacketDropped?.Invoke(_messageId);
                    _signatureBytesRemaining = MavlinkSignatureLength;
                    _state = ParserState.WaitSignatureTrailer;
                    return;
                }

                AddMavlinkFrame();
                ResetContext();
                _state = ParserState.Init;
                return;

            case ParserState.WaitSignatureTrailer:
                _signatureBytesRemaining--;
                if (_signatureBytesRemaining == 0)
                {
                    ResetContext();
                    _state = ParserState.Init;
                }
                return;
        }
    }

    private bool AddMavlinkFrame()
    {
        if (!CheckCrc())
        {
            return false;
        }

        var message = _dialect.Parse(_messageId, _payload.AsSpan(0, _payloadLength));
        if (message is null)
        {
            return false;
        }

        var frame = new MavlinkFrame(
            _version,
            (byte)_sequence,
            (byte)_systemId,
            (byte)_componentId,
            message);
        _frames.Add(frame);
        return true;
    }

    private enum ParserState
    {
        Init,
        WaitPayloadLength,
        WaitIncompatibilityFlags,
        WaitCompatibilityFlags,
        WaitPacketSequence,
        WaitSystemId,
        WaitComponentId,
        WaitMessageIdLow,
        WaitMessageIdMiddle,
        WaitMessageIdHigh,
        WaitPayloadEnd,
        WaitCrcLowByte,
        WaitCrcHighByte,
        WaitSignatureTrailer,
    }
}
