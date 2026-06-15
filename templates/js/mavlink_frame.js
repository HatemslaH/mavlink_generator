import { CrcX25 } from './crc.js';
import { MavlinkVersion } from './mavlink_version.js';

export class MavlinkFrame {
  static MAVLINK_STX_V1 = 0xfe;
  static MAVLINK_STX_V2 = 0xfd;

  constructor(version, sequence, systemId, componentId, message) {
    this.version = version;
    this.sequence = sequence;
    this.systemId = systemId;
    this.componentId = componentId;
    this.message = message;
  }

  static v1(sequence, systemId, componentId, message) {
    return new MavlinkFrame(MavlinkVersion.V1, sequence, systemId, componentId, message);
  }

  static v2(sequence, systemId, componentId, message) {
    return new MavlinkFrame(MavlinkVersion.V2, sequence, systemId, componentId, message);
  }

  serialize() {
    if (this.version === MavlinkVersion.V1) {
      return this._serializeV1();
    }
    return this._serializeV2();
  }

  _serializeV1() {
    const payload = this.message.serialize();
    const payloadLength = payload.length;
    const frame = new Uint8Array(8 + payloadLength);
    frame[0] = MavlinkFrame.MAVLINK_STX_V1;
    frame[1] = payloadLength;
    frame[2] = this.sequence;
    frame[3] = this.systemId;
    frame[4] = this.componentId;
    frame[5] = this.message.mavlinkMessageId;

    const crc = new CrcX25();
    crc.accumulate(payloadLength);
    crc.accumulate(this.sequence);
    crc.accumulate(this.systemId);
    crc.accumulate(this.componentId);
    crc.accumulate(this.message.mavlinkMessageId);

    for (let i = 0; i < payloadLength; i++) {
      frame[6 + i] = payload[i];
      crc.accumulate(payload[i]);
    }
    crc.accumulate(this.message.mavlinkCrcExtra);

    frame[frame.length - 2] = crc.crc & 0xff;
    frame[frame.length - 1] = (crc.crc >> 8) & 0xff;
    return frame;
  }

  _serializeV2() {
    const incompatibilityFlags = 0;
    const compatibilityFlags = 0;
    const payload = MavlinkFrame._trimTrailingZeros(this.message.serialize());
    const payloadLength = payload.length;
    const messageId = this.message.mavlinkMessageId;
    const messageIdBytes = [
      messageId & 0xff,
      (messageId >> 8) & 0xff,
      (messageId >> 16) & 0xff,
    ];

    const frame = new Uint8Array(12 + payloadLength);
    frame[0] = MavlinkFrame.MAVLINK_STX_V2;
    frame[1] = payloadLength;
    frame[2] = incompatibilityFlags;
    frame[3] = compatibilityFlags;
    frame[4] = this.sequence;
    frame[5] = this.systemId;
    frame[6] = this.componentId;
    frame[7] = messageIdBytes[0];
    frame[8] = messageIdBytes[1];
    frame[9] = messageIdBytes[2];

    const crc = new CrcX25();
    crc.accumulate(payloadLength);
    crc.accumulate(incompatibilityFlags);
    crc.accumulate(compatibilityFlags);
    crc.accumulate(this.sequence);
    crc.accumulate(this.systemId);
    crc.accumulate(this.componentId);
    for (const byte of messageIdBytes) {
      crc.accumulate(byte);
    }

    for (let i = 0; i < payloadLength; i++) {
      frame[10 + i] = payload[i];
      crc.accumulate(payload[i]);
    }
    crc.accumulate(this.message.mavlinkCrcExtra);

    frame[frame.length - 2] = crc.crc & 0xff;
    frame[frame.length - 1] = (crc.crc >> 8) & 0xff;
    return frame;
  }

  static _trimTrailingZeros(payload) {
    let trimmedLength = payload.length;
    while (trimmedLength > 0 && payload[trimmedLength - 1] === 0) {
      trimmedLength -= 1;
    }
    if (trimmedLength === payload.length) {
      return payload;
    }
    return payload.subarray(0, trimmedLength);
  }
}
