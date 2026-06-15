using System.Buffers.Binary;

namespace Mavlink;

public abstract class MavlinkMessage
{
    public abstract int MavlinkMessageId { get; }

    public abstract int MavlinkCrcExtra { get; }

    public abstract byte[] Serialize();

    protected static ReadOnlySpan<byte> PadPayload(ReadOnlySpan<byte> data, int length)
    {
        if (data.Length >= length)
        {
            return data;
        }

        var padded = new byte[length];
        data.CopyTo(padded);
        return padded;
    }

    public static sbyte GetInt8(ReadOnlySpan<byte> data, int offset) =>
        (sbyte)data[offset];

    public static byte GetUint8(ReadOnlySpan<byte> data, int offset) =>
        data[offset];

    public static short GetInt16(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadInt16LittleEndian(data.Slice(offset));

    public static ushort GetUint16(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(offset));

    public static int GetInt32(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadInt32LittleEndian(data.Slice(offset));

    public static uint GetUint32(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(offset));

    public static long GetInt64(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadInt64LittleEndian(data.Slice(offset));

    public static ulong GetUint64(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadUInt64LittleEndian(data.Slice(offset));

    public static float GetFloat32(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadSingleLittleEndian(data.Slice(offset));

    public static double GetFloat64(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadDoubleLittleEndian(data.Slice(offset));

    public static sbyte[] GetInt8List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new sbyte[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetInt8(data, offset + i);
        }
        return result;
    }

    public static byte[] GetUint8List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new byte[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetUint8(data, offset + i);
        }
        return result;
    }

    public static short[] GetInt16List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new short[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetInt16(data, offset + i * 2);
        }
        return result;
    }

    public static ushort[] GetUint16List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new ushort[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetUint16(data, offset + i * 2);
        }
        return result;
    }

    public static int[] GetInt32List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new int[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetInt32(data, offset + i * 4);
        }
        return result;
    }

    public static uint[] GetUint32List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new uint[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetUint32(data, offset + i * 4);
        }
        return result;
    }

    public static long[] GetInt64List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new long[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetInt64(data, offset + i * 8);
        }
        return result;
    }

    public static ulong[] GetUint64List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new ulong[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetUint64(data, offset + i * 8);
        }
        return result;
    }

    public static float[] GetFloat32List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new float[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetFloat32(data, offset + i * 4);
        }
        return result;
    }

    public static double[] GetFloat64List(ReadOnlySpan<byte> data, int offset, int length)
    {
        var result = new double[length];
        for (var i = 0; i < length; i++)
        {
            result[i] = GetFloat64(data, offset + i * 8);
        }
        return result;
    }

    public static void SetInt8(Span<byte> data, int offset, sbyte value) =>
        data[offset] = unchecked((byte)value);

    public static void SetUint8(Span<byte> data, int offset, byte value) =>
        data[offset] = value;

    public static void SetInt16(Span<byte> data, int offset, short value) =>
        BinaryPrimitives.WriteInt16LittleEndian(data.Slice(offset), value);

    public static void SetUint16(Span<byte> data, int offset, ushort value) =>
        BinaryPrimitives.WriteUInt16LittleEndian(data.Slice(offset), value);

    public static void SetInt32(Span<byte> data, int offset, int value) =>
        BinaryPrimitives.WriteInt32LittleEndian(data.Slice(offset), value);

    public static void SetUint32(Span<byte> data, int offset, uint value) =>
        BinaryPrimitives.WriteUInt32LittleEndian(data.Slice(offset), value);

    public static void SetInt64(Span<byte> data, int offset, long value) =>
        BinaryPrimitives.WriteInt64LittleEndian(data.Slice(offset), value);

    public static void SetUint64(Span<byte> data, int offset, ulong value) =>
        BinaryPrimitives.WriteUInt64LittleEndian(data.Slice(offset), value);

    public static void SetFloat32(Span<byte> data, int offset, float value) =>
        BinaryPrimitives.WriteSingleLittleEndian(data.Slice(offset), value);

    public static void SetFloat64(Span<byte> data, int offset, double value) =>
        BinaryPrimitives.WriteDoubleLittleEndian(data.Slice(offset), value);

    public static void SetInt8List(Span<byte> data, int offset, ReadOnlySpan<sbyte> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetInt8(data, offset + i, values[i]);
        }
    }

    public static void SetUint8List(Span<byte> data, int offset, ReadOnlySpan<byte> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetUint8(data, offset + i, values[i]);
        }
    }

    public static void SetInt16List(Span<byte> data, int offset, ReadOnlySpan<short> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetInt16(data, offset + i * 2, values[i]);
        }
    }

    public static void SetUint16List(Span<byte> data, int offset, ReadOnlySpan<ushort> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetUint16(data, offset + i * 2, values[i]);
        }
    }

    public static void SetInt32List(Span<byte> data, int offset, ReadOnlySpan<int> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetInt32(data, offset + i * 4, values[i]);
        }
    }

    public static void SetUint32List(Span<byte> data, int offset, ReadOnlySpan<uint> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetUint32(data, offset + i * 4, values[i]);
        }
    }

    public static void SetInt64List(Span<byte> data, int offset, ReadOnlySpan<long> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetInt64(data, offset + i * 8, values[i]);
        }
    }

    public static void SetUint64List(Span<byte> data, int offset, ReadOnlySpan<ulong> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetUint64(data, offset + i * 8, values[i]);
        }
    }

    public static void SetFloat32List(Span<byte> data, int offset, ReadOnlySpan<float> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetFloat32(data, offset + i * 4, values[i]);
        }
    }

    public static void SetFloat64List(Span<byte> data, int offset, ReadOnlySpan<double> values)
    {
        for (var i = 0; i < values.Length; i++)
        {
            SetFloat64(data, offset + i * 8, values[i]);
        }
    }
}
