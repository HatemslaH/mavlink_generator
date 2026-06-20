import { MavParamType } from '../mavlink.js';

/** Byte-wise parameter value encoding per MAVLink parameter protocol. */
export class ParamCodec {
  static encodeInt8(value) {
    return ParamCodec._encodeInt32(value);
  }

  static decodeInt8(encoded) {
    return ParamCodec.decodeInt32(encoded);
  }

  static encodeUint8(value) {
    return ParamCodec._encodeUint32(value);
  }

  static decodeUint8(encoded) {
    return ParamCodec.decodeUint32(encoded);
  }

  static encodeInt16(value) {
    return ParamCodec._encodeInt32(value);
  }

  static decodeInt16(encoded) {
    return ParamCodec.decodeInt32(encoded);
  }

  static encodeUint16(value) {
    return ParamCodec._encodeUint32(value);
  }

  static decodeUint16(encoded) {
    return ParamCodec.decodeUint32(encoded);
  }

  static encodeInt32(value) {
    return ParamCodec._encodeInt32(value);
  }

  static decodeInt32(encoded) {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setFloat32(0, encoded, true);
    return bytes.getInt32(0, true);
  }

  static encodeUint32(value) {
    return ParamCodec._encodeUint32(value);
  }

  static decodeUint32(encoded) {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setFloat32(0, encoded, true);
    return bytes.getUint32(0, true);
  }

  static encodeFloat(value) {
    return value;
  }

  static decodeFloat(encoded) {
    return encoded;
  }

  static encodeValue(value, type) {
    switch (type) {
      case MavParamType.MAV_PARAM_TYPE_UINT8:
        return ParamCodec.encodeUint8(Number(value));
      case MavParamType.MAV_PARAM_TYPE_INT8:
        return ParamCodec.encodeInt8(Number(value));
      case MavParamType.MAV_PARAM_TYPE_UINT16:
        return ParamCodec.encodeUint16(Number(value));
      case MavParamType.MAV_PARAM_TYPE_INT16:
        return ParamCodec.encodeInt16(Number(value));
      case MavParamType.MAV_PARAM_TYPE_INT32:
        return ParamCodec.encodeInt32(Number(value));
      case MavParamType.MAV_PARAM_TYPE_UINT32:
        return ParamCodec.encodeUint32(Number(value));
      case MavParamType.MAV_PARAM_TYPE_REAL32:
        return ParamCodec.encodeFloat(Number(value));
      default:
        return Number(value);
    }
  }

  static decodeValue(encoded, type) {
    switch (type) {
      case MavParamType.MAV_PARAM_TYPE_UINT8:
        return ParamCodec.decodeUint8(encoded);
      case MavParamType.MAV_PARAM_TYPE_INT8:
        return ParamCodec.decodeInt8(encoded);
      case MavParamType.MAV_PARAM_TYPE_UINT16:
        return ParamCodec.decodeUint16(encoded);
      case MavParamType.MAV_PARAM_TYPE_INT16:
        return ParamCodec.decodeInt16(encoded);
      case MavParamType.MAV_PARAM_TYPE_INT32:
        return ParamCodec.decodeInt32(encoded);
      case MavParamType.MAV_PARAM_TYPE_UINT32:
        return ParamCodec.decodeUint32(encoded);
      case MavParamType.MAV_PARAM_TYPE_REAL32:
        return ParamCodec.decodeFloat(encoded);
      default:
        return encoded;
    }
  }

  /** Encode a short ASCII parameter name into a MAVLink `char[16]` field. */
  static paramIdFromString(name) {
    const id = [];
    for (const ch of name.slice(0, 16)) {
      id.push(ch.charCodeAt(0));
    }
    while (id.length < 16) {
      id.push(0);
    }
    return id;
  }

  /** Decode a MAVLink `char[16]` parameter id to a string. */
  static paramIdToString(id) {
    const end = id.findIndex((c) => c === 0);
    const slice = end === -1 ? id : id.slice(0, end);
    return String.fromCharCode(...slice);
  }

  static _encodeInt32(value) {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setInt32(0, value, true);
    return bytes.getFloat32(0, true);
  }

  static _encodeUint32(value) {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setUint32(0, value, true);
    return bytes.getFloat32(0, true);
  }
}
