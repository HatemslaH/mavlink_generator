export class MavlinkMessage {
  get mavlinkMessageId() {
    throw new Error('not implemented');
  }

  get mavlinkCrcExtra() {
    throw new Error('not implemented');
  }

  serialize() {
    throw new Error('not implemented');
  }

  static _view(data, offset = 0) {
    return new DataView(data.buffer, data.byteOffset + offset, data.byteLength - offset);
  }

  static _getInt8(data, offset) {
    return this._view(data).getInt8(offset);
  }

  static _getUint8(data, offset) {
    return this._view(data).getUint8(offset);
  }

  static _getInt16(data, offset) {
    return this._view(data).getInt16(offset, true);
  }

  static _getUint16(data, offset) {
    return this._view(data).getUint16(offset, true);
  }

  static _getInt32(data, offset) {
    return this._view(data).getInt32(offset, true);
  }

  static _getUint32(data, offset) {
    return this._view(data).getUint32(offset, true);
  }

  static _getInt64(data, offset) {
    return Number(this._view(data).getBigInt64(offset, true));
  }

  static _getUint64(data, offset) {
    return Number(this._view(data).getBigUint64(offset, true));
  }

  static _getFloat32(data, offset) {
    return this._view(data).getFloat32(offset, true);
  }

  static _getFloat64(data, offset) {
    return this._view(data).getFloat64(offset, true);
  }

  static _getInt8List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getInt8(data, offset + i));
    }
    return result;
  }

  static _getUint8List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getUint8(data, offset + i));
    }
    return result;
  }

  static _getInt16List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getInt16(data, offset + i * 2));
    }
    return result;
  }

  static _getUint16List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getUint16(data, offset + i * 2));
    }
    return result;
  }

  static _getInt32List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getInt32(data, offset + i * 4));
    }
    return result;
  }

  static _getUint32List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getUint32(data, offset + i * 4));
    }
    return result;
  }

  static _getInt64List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getInt64(data, offset + i * 8));
    }
    return result;
  }

  static _getUint64List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getUint64(data, offset + i * 8));
    }
    return result;
  }

  static _getFloat32List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getFloat32(data, offset + i * 4));
    }
    return result;
  }

  static _getFloat64List(data, offset, length) {
    const result = [];
    for (let i = 0; i < length; i++) {
      result.push(this._getFloat64(data, offset + i * 8));
    }
    return result;
  }

  static _setInt8(data, offset, value) {
    this._view(data).setInt8(offset, value);
  }

  static _setUint8(data, offset, value) {
    this._view(data).setUint8(offset, value);
  }

  static _setInt16(data, offset, value) {
    this._view(data).setInt16(offset, value, true);
  }

  static _setUint16(data, offset, value) {
    this._view(data).setUint16(offset, value, true);
  }

  static _setInt32(data, offset, value) {
    this._view(data).setInt32(offset, value, true);
  }

  static _setUint32(data, offset, value) {
    this._view(data).setUint32(offset, value, true);
  }

  static _setInt64(data, offset, value) {
    this._view(data).setBigInt64(offset, BigInt(value), true);
  }

  static _setUint64(data, offset, value) {
    this._view(data).setBigUint64(offset, BigInt(value), true);
  }

  static _setFloat32(data, offset, value) {
    this._view(data).setFloat32(offset, value, true);
  }

  static _setFloat64(data, offset, value) {
    this._view(data).setFloat64(offset, value, true);
  }

  static _setInt8List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setInt8(data, offset + i, values[i]);
    }
  }

  static _setUint8List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setUint8(data, offset + i, values[i]);
    }
  }

  static _setInt16List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setInt16(data, offset + i * 2, values[i]);
    }
  }

  static _setUint16List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setUint16(data, offset + i * 2, values[i]);
    }
  }

  static _setInt32List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setInt32(data, offset + i * 4, values[i]);
    }
  }

  static _setUint32List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setUint32(data, offset + i * 4, values[i]);
    }
  }

  static _setInt64List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setInt64(data, offset + i * 8, values[i]);
    }
  }

  static _setUint64List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setUint64(data, offset + i * 8, values[i]);
    }
  }

  static _setFloat32List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setFloat32(data, offset + i * 4, values[i]);
    }
  }

  static _setFloat64List(data, offset, values) {
    for (let i = 0; i < values.length; i++) {
      this._setFloat64(data, offset + i * 8, values[i]);
    }
  }
}
