namespace Mavlink;

public class CrcX25
{
    private const ushort X25InitCrc = 0xFFFF;
    private ushort _crc = X25InitCrc;

    public ushort Crc => (ushort)(_crc & 0xFFFF);

    public void Accumulate(byte value)
    {
        var tmp = (byte)(value ^ (_crc & 0xFF));
        tmp = (byte)(tmp ^ ((tmp << 4) & 0xFF));
        _crc = (ushort)(
            (_crc >> 8)
            ^ ((tmp << 8) & 0xFFFF)
            ^ ((tmp << 3) & 0xFFFF)
            ^ (tmp >> 4)
        );
    }

    public void AccumulateString(string text)
    {
        foreach (var codeUnit in System.Text.Encoding.ASCII.GetBytes(text))
        {
            Accumulate(codeUnit);
        }
    }
}
