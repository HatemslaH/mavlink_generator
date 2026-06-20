import 'dart:typed_data';

import '../mavlink.dart';

/// Byte-wise parameter value encoding per MAVLink parameter protocol.
///
/// See https://mavlink.io/en/services/parameter.html
class ParamCodec {
  ParamCodec._();

  static double encodeInt8(int value) => _encodeInt32(value);

  static int decodeInt8(double encoded) => decodeInt32(encoded);

  static double encodeUint8(int value) => _encodeUint32(value);

  static int decodeUint8(double encoded) => decodeUint32(encoded);

  static double encodeInt16(int value) => _encodeInt32(value);

  static int decodeInt16(double encoded) => decodeInt32(encoded);

  static double encodeUint16(int value) => _encodeUint32(value);

  static int decodeUint16(double encoded) => decodeUint32(encoded);

  static double encodeInt32(int value) => _encodeInt32(value);

  static int decodeInt32(double encoded) {
    final bytes = ByteData(4)..setFloat32(0, encoded, Endian.little);
    return bytes.getInt32(0, Endian.little);
  }

  static double encodeUint32(int value) => _encodeUint32(value);

  static int decodeUint32(double encoded) {
    final bytes = ByteData(4)..setFloat32(0, encoded, Endian.little);
    return bytes.getUint32(0, Endian.little);
  }

  static double encodeFloat(double value) => value;

  static double decodeFloat(double encoded) => encoded;

  static double encodeValue(num value, MavParamType type) {
    return switch (type) {
      MavParamType.mavParamTypeUint8 => encodeUint8(value.toInt()),
      MavParamType.mavParamTypeInt8 => encodeInt8(value.toInt()),
      MavParamType.mavParamTypeUint16 => encodeUint16(value.toInt()),
      MavParamType.mavParamTypeInt16 => encodeInt16(value.toInt()),
      MavParamType.mavParamTypeInt32 => encodeInt32(value.toInt()),
      MavParamType.mavParamTypeUint32 => encodeUint32(value.toInt()),
      MavParamType.mavParamTypeReal32 => encodeFloat(value.toDouble()),
      _ => value.toDouble(),
    };
  }

  static num decodeValue(double encoded, MavParamType type) {
    return switch (type) {
      MavParamType.mavParamTypeUint8 => decodeUint8(encoded),
      MavParamType.mavParamTypeInt8 => decodeInt8(encoded),
      MavParamType.mavParamTypeUint16 => decodeUint16(encoded),
      MavParamType.mavParamTypeInt16 => decodeInt16(encoded),
      MavParamType.mavParamTypeInt32 => decodeInt32(encoded),
      MavParamType.mavParamTypeUint32 => decodeUint32(encoded),
      MavParamType.mavParamTypeReal32 => decodeFloat(encoded),
      _ => encoded,
    };
  }

  /// Encode a short ASCII parameter name into a MAVLink `char[16]` field.
  static List<char> paramIdFromString(String name) {
    final id = <char>[];
    for (final unit in name.codeUnits.take(16)) {
      id.add(unit);
    }
    while (id.length < 16) {
      id.add(0);
    }
    return id;
  }

  /// Decode a MAVLink `char[16]` parameter id to a string.
  static String paramIdToString(List<char> id) {
    final end = id.indexWhere((c) => c == 0);
    final slice = end == -1 ? id : id.sublist(0, end);
    return String.fromCharCodes(slice);
  }

  static double _encodeInt32(int value) {
    final bytes = ByteData(4)..setInt32(0, value, Endian.little);
    return bytes.getFloat32(0, Endian.little);
  }

  static double _encodeUint32(int value) {
    final bytes = ByteData(4)..setUint32(0, value, Endian.little);
    return bytes.getFloat32(0, Endian.little);
  }
}
