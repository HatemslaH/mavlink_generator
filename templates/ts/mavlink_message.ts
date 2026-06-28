export abstract class MavlinkMessage {
  abstract get mavlinkMessageId(): number;
  abstract get mavlinkCrcExtra(): number;
  abstract serialize(): Uint8Array;

  /** Message id check that survives duplicate ESM module instances. */
  static isMessageOf<T extends MavlinkMessage>(
    message: MavlinkMessage,
    messageClass: { readonly MSG_ID: number },
  ): message is T {
    return message.mavlinkMessageId === messageClass.MSG_ID;
  }

  private static view(data: Uint8Array): DataView {
    return new DataView(data.buffer, data.byteOffset, data.byteLength);
  }

  protected static getInt8(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getInt8(offset);
  }

  protected static getUint8(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getUint8(offset);
  }

  protected static getInt16(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getInt16(offset, true);
  }

  protected static getUint16(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getUint16(offset, true);
  }

  protected static getInt32(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getInt32(offset, true);
  }

  protected static getUint32(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getUint32(offset, true);
  }

  protected static getInt64(data: Uint8Array, offset: number): number {
    return Number(MavlinkMessage.view(data).getBigInt64(offset, true));
  }

  protected static getUint64(data: Uint8Array, offset: number): number {
    return Number(MavlinkMessage.view(data).getBigUint64(offset, true));
  }

  protected static getFloat32(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getFloat32(offset, true);
  }

  protected static getFloat64(data: Uint8Array, offset: number): number {
    return MavlinkMessage.view(data).getFloat64(offset, true);
  }

  protected static getInt8List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) => MavlinkMessage.getInt8(data, offset + i));
  }

  protected static getUint8List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) => MavlinkMessage.getUint8(data, offset + i));
  }

  protected static getInt16List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getInt16(data, offset + i * 2),
    );
  }

  protected static getUint16List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getUint16(data, offset + i * 2),
    );
  }

  protected static getInt32List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getInt32(data, offset + i * 4),
    );
  }

  protected static getUint32List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getUint32(data, offset + i * 4),
    );
  }

  protected static getInt64List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getInt64(data, offset + i * 8),
    );
  }

  protected static getUint64List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getUint64(data, offset + i * 8),
    );
  }

  protected static getFloat32List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getFloat32(data, offset + i * 4),
    );
  }

  protected static getFloat64List(
    data: Uint8Array,
    offset: number,
    length: number,
  ): number[] {
    return Array.from({ length }, (_, i) =>
      MavlinkMessage.getFloat64(data, offset + i * 8),
    );
  }

  protected static setInt8(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setInt8(offset, value);
  }

  protected static setUint8(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setUint8(offset, value);
  }

  protected static setInt16(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setInt16(offset, value, true);
  }

  protected static setUint16(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setUint16(offset, value, true);
  }

  protected static setInt32(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setInt32(offset, value, true);
  }

  protected static setUint32(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setUint32(offset, value, true);
  }

  protected static setInt64(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setBigInt64(offset, BigInt(value), true);
  }

  protected static setUint64(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setBigUint64(offset, BigInt(value), true);
  }

  protected static setFloat32(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setFloat32(offset, value, true);
  }

  protected static setFloat64(data: Uint8Array, offset: number, value: number): void {
    MavlinkMessage.view(data).setFloat64(offset, value, true);
  }

  protected static setInt8List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setInt8(data, offset + i, values[i]!);
    }
  }

  protected static setUint8List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setUint8(data, offset + i, values[i]!);
    }
  }

  protected static setInt16List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setInt16(data, offset + i * 2, values[i]!);
    }
  }

  protected static setUint16List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setUint16(data, offset + i * 2, values[i]!);
    }
  }

  protected static setInt32List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setInt32(data, offset + i * 4, values[i]!);
    }
  }

  protected static setUint32List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setUint32(data, offset + i * 4, values[i]!);
    }
  }

  protected static setInt64List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setInt64(data, offset + i * 8, values[i]!);
    }
  }

  protected static setUint64List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setUint64(data, offset + i * 8, values[i]!);
    }
  }

  protected static setFloat32List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setFloat32(data, offset + i * 4, values[i]!);
    }
  }

  protected static setFloat64List(
    data: Uint8Array,
    offset: number,
    values: number[],
  ): void {
    for (let i = 0; i < values.length; i++) {
      MavlinkMessage.setFloat64(data, offset + i * 8, values[i]!);
    }
  }
}
