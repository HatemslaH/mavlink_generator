using System.Buffers.Binary;
using Mavlink.Dialects;

namespace Mavlink;

/// <summary>
/// Byte-wise parameter value encoding per MAVLink parameter protocol.
/// </summary>
/// <remarks>See https://mavlink.io/en/services/parameter.html</remarks>
public static class ParamCodec
{
    public static float EncodeInt8(int value) => EncodeInt32(value);

    public static int DecodeInt8(float encoded) => DecodeInt32(encoded);

    public static float EncodeUint8(int value) => EncodeUint32(value);

    public static int DecodeUint8(float encoded) => DecodeUint32(encoded);

    public static float EncodeInt16(int value) => EncodeInt32(value);

    public static int DecodeInt16(float encoded) => DecodeInt32(encoded);

    public static float EncodeUint16(int value) => EncodeUint32(value);

    public static int DecodeUint16(float encoded) => DecodeUint32(encoded);

    public static float EncodeInt32(int value)
    {
        Span<byte> bytes = stackalloc byte[4];
        BinaryPrimitives.WriteInt32LittleEndian(bytes, value);
        return BinaryPrimitives.ReadSingleLittleEndian(bytes);
    }

    public static int DecodeInt32(float encoded)
    {
        Span<byte> bytes = stackalloc byte[4];
        BinaryPrimitives.WriteSingleLittleEndian(bytes, encoded);
        return BinaryPrimitives.ReadInt32LittleEndian(bytes);
    }

    public static float EncodeUint32(int value)
    {
        Span<byte> bytes = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(bytes, (uint)value);
        return BinaryPrimitives.ReadSingleLittleEndian(bytes);
    }

    public static int DecodeUint32(float encoded)
    {
        Span<byte> bytes = stackalloc byte[4];
        BinaryPrimitives.WriteSingleLittleEndian(bytes, encoded);
        return (int)BinaryPrimitives.ReadUInt32LittleEndian(bytes);
    }

    public static float EncodeFloat(float value) => value;

    public static float DecodeFloat(float encoded) => encoded;

    public static float EncodeValue(decimal value, MavParamType type) =>
        type switch
        {
            MavParamType.MAV_PARAM_TYPE_UINT8 => EncodeUint8((int)value),
            MavParamType.MAV_PARAM_TYPE_INT8 => EncodeInt8((int)value),
            MavParamType.MAV_PARAM_TYPE_UINT16 => EncodeUint16((int)value),
            MavParamType.MAV_PARAM_TYPE_INT16 => EncodeInt16((int)value),
            MavParamType.MAV_PARAM_TYPE_INT32 => EncodeInt32((int)value),
            MavParamType.MAV_PARAM_TYPE_UINT32 => EncodeUint32((int)value),
            MavParamType.MAV_PARAM_TYPE_REAL32 => EncodeFloat((float)value),
            _ => (float)value,
        };

    public static decimal DecodeValue(float encoded, MavParamType type) =>
        type switch
        {
            MavParamType.MAV_PARAM_TYPE_UINT8 => DecodeUint8(encoded),
            MavParamType.MAV_PARAM_TYPE_INT8 => DecodeInt8(encoded),
            MavParamType.MAV_PARAM_TYPE_UINT16 => DecodeUint16(encoded),
            MavParamType.MAV_PARAM_TYPE_INT16 => DecodeInt16(encoded),
            MavParamType.MAV_PARAM_TYPE_INT32 => DecodeInt32(encoded),
            MavParamType.MAV_PARAM_TYPE_UINT32 => DecodeUint32(encoded),
            MavParamType.MAV_PARAM_TYPE_REAL32 => (decimal)DecodeFloat(encoded),
            _ => (decimal)encoded,
        };

    /// <summary>Encode a short ASCII parameter name into a MAVLink <c>char[16]</c> field.</summary>
    public static byte[] ParamIdFromString(string name)
    {
        var id = new byte[16];
        var bytes = System.Text.Encoding.ASCII.GetBytes(name);
        Array.Copy(bytes, 0, id, 0, Math.Min(bytes.Length, 16));
        return id;
    }

    /// <summary>Decode a MAVLink <c>char[16]</c> parameter id to a string.</summary>
    public static string ParamIdToString(byte[] id)
    {
        var end = Array.FindIndex(id, value => value == 0);
        if (end < 0)
        {
            end = id.Length;
        }

        return System.Text.Encoding.ASCII.GetString(id, 0, end);
    }
}
