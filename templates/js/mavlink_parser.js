import { CrcX25 } from './crc.js';
import { MavlinkFrame } from './mavlink_frame.js';
import { MavlinkVersion } from './mavlink_version.js';

const ParserState = {
  INIT: 0,
  WAIT_PAYLOAD_LENGTH: 1,
  WAIT_INCOMPATIBILITY_FLAGS: 2,
  WAIT_COMPATIBILITY_FLAGS: 3,
  WAIT_PACKET_SEQUENCE: 4,
  WAIT_SYSTEM_ID: 5,
  WAIT_COMPONENT_ID: 6,
  WAIT_MESSAGE_ID_LOW: 7,
  WAIT_MESSAGE_ID_MIDDLE: 8,
  WAIT_MESSAGE_ID_HIGH: 9,
  WAIT_PAYLOAD_END: 10,
  WAIT_CRC_LOW_BYTE: 11,
  WAIT_CRC_HIGH_BYTE: 12,
  WAIT_SIGNATURE_TRAILER: 13,
};

export class MavlinkParser {
  static MAVLINK_MAXIMUM_PAYLOAD_SIZE = 255;
  static MAVLINK_IFLAG_SIGNED = 0x01;
  static MAVLINK_SIGNATURE_LENGTH = 13;

  constructor(dialect, onSignedPacketDropped = null) {
    this._dialect = dialect;
    this.onSignedPacketDropped = onSignedPacketDropped;
    this._frames = [];
    this._resetContext();
    this._state = ParserState.INIT;
  }

  get frames() {
    return this._frames;
  }

  _resetContext() {
    this._version = MavlinkVersion.V1;
    this._payloadLength = -1;
    this._incompatibilityFlags = -1;
    this._compatibilityFlags = -1;
    this._sequence = -1;
    this._systemId = -1;
    this._componentId = -1;
    this._messageIdLow = -1;
    this._messageIdMiddle = -1;
    this._messageIdHigh = -1;
    this._messageId = -1;
    this._payload = new Uint8Array(MavlinkParser.MAVLINK_MAXIMUM_PAYLOAD_SIZE);
    this._payloadCursor = -1;
    this._crcLowByte = -1;
    this._crcHighByte = -1;
    this._signatureBytesRemaining = 0;
  }

  _checkCrc() {
    let header;
    if (this._version === MavlinkVersion.V1) {
      header = [
        this._payloadLength,
        this._sequence,
        this._systemId,
        this._componentId,
        this._messageId,
      ];
    } else {
      header = [
        this._payloadLength,
        this._incompatibilityFlags,
        this._compatibilityFlags,
        this._sequence,
        this._systemId,
        this._componentId,
        this._messageIdLow,
        this._messageIdMiddle,
        this._messageIdHigh,
      ];
    }

    const crc = new CrcX25();
    for (const value of header) {
      crc.accumulate(value & 0xff);
    }
    for (let i = 0; i < this._payloadLength; i++) {
      crc.accumulate(this._payload[i] & 0xff);
    }

    const crcExt = this._dialect.crcExtra(this._messageId);
    if (crcExt === -1) {
      return false;
    }
    crc.accumulate(crcExt);
    return crc.crc === ((this._crcHighByte << 8) ^ this._crcLowByte);
  }

  parse(data) {
    for (let i = 0; i < data.length; i++) {
      this._parseByte(data[i]);
    }
  }

  _parseByte(byte) {
    if (this._state === ParserState.INIT) {
      if (byte === MavlinkFrame.MAVLINK_STX_V1) {
        this._version = MavlinkVersion.V1;
        this._state = ParserState.WAIT_PAYLOAD_LENGTH;
      } else if (byte === MavlinkFrame.MAVLINK_STX_V2) {
        this._version = MavlinkVersion.V2;
        this._state = ParserState.WAIT_PAYLOAD_LENGTH;
      }
      return;
    }

    if (this._state === ParserState.WAIT_PAYLOAD_LENGTH) {
      this._payloadLength = byte;
      if (this._version === MavlinkVersion.V1) {
        this._state = ParserState.WAIT_PACKET_SEQUENCE;
      } else {
        this._state = ParserState.WAIT_INCOMPATIBILITY_FLAGS;
      }
      return;
    }

    if (this._state === ParserState.WAIT_INCOMPATIBILITY_FLAGS) {
      this._incompatibilityFlags = byte;
      this._state = ParserState.WAIT_COMPATIBILITY_FLAGS;
      return;
    }

    if (this._state === ParserState.WAIT_COMPATIBILITY_FLAGS) {
      this._compatibilityFlags = byte;
      this._state = ParserState.WAIT_PACKET_SEQUENCE;
      return;
    }

    if (this._state === ParserState.WAIT_PACKET_SEQUENCE) {
      this._sequence = byte;
      this._state = ParserState.WAIT_SYSTEM_ID;
      return;
    }

    if (this._state === ParserState.WAIT_SYSTEM_ID) {
      this._systemId = byte;
      this._state = ParserState.WAIT_COMPONENT_ID;
      return;
    }

    if (this._state === ParserState.WAIT_COMPONENT_ID) {
      this._componentId = byte;
      if (this._version === MavlinkVersion.V1) {
        this._state = ParserState.WAIT_MESSAGE_ID_HIGH;
      } else {
        this._state = ParserState.WAIT_MESSAGE_ID_LOW;
      }
      return;
    }

    if (this._state === ParserState.WAIT_MESSAGE_ID_LOW) {
      this._messageIdLow = byte;
      this._state = ParserState.WAIT_MESSAGE_ID_MIDDLE;
      return;
    }

    if (this._state === ParserState.WAIT_MESSAGE_ID_MIDDLE) {
      this._messageIdMiddle = byte;
      this._state = ParserState.WAIT_MESSAGE_ID_HIGH;
      return;
    }

    if (this._state === ParserState.WAIT_MESSAGE_ID_HIGH) {
      if (this._version === MavlinkVersion.V1) {
        this._messageId = byte;
      } else {
        this._messageIdHigh = byte;
        this._messageId =
          (this._messageIdHigh << 16) ^
          (this._messageIdMiddle << 8) ^
          this._messageIdLow;
      }
      if (this._payloadLength === 0) {
        this._state = ParserState.WAIT_CRC_LOW_BYTE;
      } else {
        this._state = ParserState.WAIT_PAYLOAD_END;
        this._payloadCursor = 0;
      }
      return;
    }

    if (this._state === ParserState.WAIT_PAYLOAD_END) {
      if (this._payloadCursor < this._payloadLength) {
        this._payload[this._payloadCursor] = byte;
        this._payloadCursor += 1;
      }
      if (this._payloadCursor === this._payloadLength) {
        this._state = ParserState.WAIT_CRC_LOW_BYTE;
      }
      return;
    }

    if (this._state === ParserState.WAIT_CRC_LOW_BYTE) {
      this._crcLowByte = byte;
      this._state = ParserState.WAIT_CRC_HIGH_BYTE;
      return;
    }

    if (this._state === ParserState.WAIT_CRC_HIGH_BYTE) {
      this._crcHighByte = byte;
      if (
        this._version === MavlinkVersion.V2 &&
        (this._incompatibilityFlags & MavlinkParser.MAVLINK_IFLAG_SIGNED) !== 0
      ) {
        if (this.onSignedPacketDropped !== null) {
          this.onSignedPacketDropped(this._messageId);
        }
        this._signatureBytesRemaining = MavlinkParser.MAVLINK_SIGNATURE_LENGTH;
        this._state = ParserState.WAIT_SIGNATURE_TRAILER;
        return;
      }

      this._addMavlinkFrame();
      this._resetContext();
      this._state = ParserState.INIT;
      return;
    }

    if (this._state === ParserState.WAIT_SIGNATURE_TRAILER) {
      this._signatureBytesRemaining -= 1;
      if (this._signatureBytesRemaining === 0) {
        this._resetContext();
        this._state = ParserState.INIT;
      }
    }
  }

  _addMavlinkFrame() {
    if (!this._checkCrc()) {
      return false;
    }

    const message = this._dialect.parse(
      this._messageId,
      this._payload.subarray(0, this._payloadLength),
    );
    if (message === null) {
      return false;
    }

    const frame = new MavlinkFrame(
      this._version,
      this._sequence,
      this._systemId,
      this._componentId,
      message,
    );
    this._frames.push(frame);
    return true;
  }
}
