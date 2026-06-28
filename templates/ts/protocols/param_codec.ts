import { MavParamType } from '../mavlink';

/** Byte-wise parameter value encoding per MAVLink parameter protocol. */
export class ParamCodec {
  private constructor() {}

  static encodeInt8(value: number): number {
    return ParamCodec.encodeInt32(value);
  }

  static decodeInt8(encoded: number): number {
    return ParamCodec.decodeInt32(encoded);
  }

  static encodeUint8(value: number): number {
    return ParamCodec.encodeUint32(value);
  }

  static decodeUint8(encoded: number): number {
    return ParamCodec.decodeUint32(encoded);
  }

  static encodeInt16(value: number): number {
    return ParamCodec.encodeInt32(value);
  }

  static decodeInt16(encoded: number): number {
    return ParamCodec.decodeInt32(encoded);
  }

  static encodeUint16(value: number): number {
    return ParamCodec.encodeUint32(value);
  }

  static decodeUint16(encoded: number): number {
    return ParamCodec.decodeUint32(encoded);
  }

  static encodeInt32(value: number): number {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setInt32(0, value, true);
    return bytes.getFloat32(0, true);
  }

  static decodeInt32(encoded: number): number {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setFloat32(0, encoded, true);
    return bytes.getInt32(0, true);
  }

  static encodeUint32(value: number): number {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setUint32(0, value >>> 0, true);
    return bytes.getFloat32(0, true);
  }

  static decodeUint32(encoded: number): number {
    const bytes = new DataView(new ArrayBuffer(4));
    bytes.setFloat32(0, encoded, true);
    return bytes.getUint32(0, true);
  }

  static encodeFloat(value: number): number {
    return value;
  }

  static decodeFloat(encoded: number): number {
    return encoded;
  }

  static encodeValue(value: number, type: MavParamType): number {
    switch (type) {
      case MavParamType.MAV_PARAM_TYPE_UINT8:
        return ParamCodec.encodeUint8(value);
      case MavParamType.MAV_PARAM_TYPE_INT8:
        return ParamCodec.encodeInt8(value);
      case MavParamType.MAV_PARAM_TYPE_UINT16:
        return ParamCodec.encodeUint16(value);
      case MavParamType.MAV_PARAM_TYPE_INT16:
        return ParamCodec.encodeInt16(value);
      case MavParamType.MAV_PARAM_TYPE_INT32:
        return ParamCodec.encodeInt32(value);
      case MavParamType.MAV_PARAM_TYPE_UINT32:
        return ParamCodec.encodeUint32(value);
      case MavParamType.MAV_PARAM_TYPE_REAL32:
        return ParamCodec.encodeFloat(value);
      default:
        return value;
    }
  }

  static decodeValue(encoded: number, type: MavParamType): number {
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
  static paramIdFromString(name: string): number[] {
    const id: number[] = [];
    for (const codeUnit of name.slice(0, 16)) {
      id.push(codeUnit.charCodeAt(0));
    }
    while (id.length < 16) {
      id.push(0);
    }
    return id;
  }

  /** Decode a MAVLink `char[16]` parameter id to a string. */
  static paramIdToString(id: number[]): string {
    const end = id.findIndex((value) => value === 0);
    const slice = end === -1 ? id : id.slice(0, end);
    return String.fromCharCode(...slice);
  }
}
