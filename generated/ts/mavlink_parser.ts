import { CrcX25 } from './crc';
import type { MavlinkDialect } from './mavlink_dialect';
import { MavlinkFrame } from './mavlink_frame';
import { MavlinkVersion } from './mavlink_version';

enum ParserState {
  Init,
  WaitPayloadLength,
  WaitIncompatibilityFlags,
  WaitCompatibilityFlags,
  WaitPacketSequence,
  WaitSystemId,
  WaitComponentId,
  WaitMessageIdLow,
  WaitMessageIdMiddle,
  WaitMessageIdHigh,
  WaitPayloadEnd,
  WaitCrcLowByte,
  WaitCrcHighByte,
  WaitSignatureTrailer,
}

export class MavlinkParser {
  private static readonly MAVLINK_MAXIMUM_PAYLOAD_SIZE = 255;
  private static readonly MAVLINK_IFLAG_SIGNED = 0x01;
  private static readonly MAVLINK_SIGNATURE_LENGTH = 13;

  private readonly _dialect: MavlinkDialect;
  private readonly _frames: MavlinkFrame[] = [];
  private _state = ParserState.Init;
  private _version = MavlinkVersion.V1;
  private _payloadLength = -1;
  private _incompatibilityFlags = -1;
  private _compatibilityFlags = -1;
  private _sequence = -1;
  private _systemId = -1;
  private _componentId = -1;
  private _messageIdLow = -1;
  private _messageIdMiddle = -1;
  private _messageIdHigh = -1;
  private _messageId = -1;
  private readonly _payload = new Uint8Array(
    MavlinkParser.MAVLINK_MAXIMUM_PAYLOAD_SIZE,
  );
  private _payloadCursor = -1;
  private _crcLowByte = -1;
  private _crcHighByte = -1;
  private _signatureBytesRemaining = 0;

  onSignedPacketDropped: ((messageId: number) => void) | null = null;

  constructor(
    dialect: MavlinkDialect,
    onSignedPacketDropped: ((messageId: number) => void) | null = null,
  ) {
    this._dialect = dialect;
    this.onSignedPacketDropped = onSignedPacketDropped;
  }

  get frames(): readonly MavlinkFrame[] {
    return this._frames;
  }

  parse(data: Uint8Array): void {
    for (const byte of data) {
      this.parseByte(byte);
    }
  }

  private resetContext(): void {
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
    this._payload.fill(0);
    this._payloadCursor = -1;
    this._crcLowByte = -1;
    this._crcHighByte = -1;
    this._signatureBytesRemaining = 0;
  }

  private checkCrc(): boolean {
    const header =
      this._version === MavlinkVersion.V1
        ? [
            this._payloadLength,
            this._sequence,
            this._systemId,
            this._componentId,
            this._messageId,
          ]
        : [
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

    const crc = new CrcX25();
    for (const value of header) {
      crc.accumulate(value & 0xff);
    }
    for (let i = 0; i < this._payloadLength; i++) {
      crc.accumulate(this._payload[i]! & 0xff);
    }

    const crcExt = this._dialect.crcExtra(this._messageId);
    if (crcExt === -1) {
      return false;
    }
    crc.accumulate(crcExt);
    return crc.crc === ((this._crcHighByte << 8) ^ this._crcLowByte);
  }

  private parseByte(byte: number): void {
    switch (this._state) {
      case ParserState.Init:
        if (byte === MavlinkFrame.MAVLINK_STX_V1) {
          this._version = MavlinkVersion.V1;
          this._state = ParserState.WaitPayloadLength;
        } else if (byte === MavlinkFrame.MAVLINK_STX_V2) {
          this._version = MavlinkVersion.V2;
          this._state = ParserState.WaitPayloadLength;
        }
        return;

      case ParserState.WaitPayloadLength:
        this._payloadLength = byte;
        this._state =
          this._version === MavlinkVersion.V1
            ? ParserState.WaitPacketSequence
            : ParserState.WaitIncompatibilityFlags;
        return;

      case ParserState.WaitIncompatibilityFlags:
        this._incompatibilityFlags = byte;
        this._state = ParserState.WaitCompatibilityFlags;
        return;

      case ParserState.WaitCompatibilityFlags:
        this._compatibilityFlags = byte;
        this._state = ParserState.WaitPacketSequence;
        return;

      case ParserState.WaitPacketSequence:
        this._sequence = byte;
        this._state = ParserState.WaitSystemId;
        return;

      case ParserState.WaitSystemId:
        this._systemId = byte;
        this._state = ParserState.WaitComponentId;
        return;

      case ParserState.WaitComponentId:
        this._componentId = byte;
        this._state =
          this._version === MavlinkVersion.V1
            ? ParserState.WaitMessageIdHigh
            : ParserState.WaitMessageIdLow;
        return;

      case ParserState.WaitMessageIdLow:
        this._messageIdLow = byte;
        this._state = ParserState.WaitMessageIdMiddle;
        return;

      case ParserState.WaitMessageIdMiddle:
        this._messageIdMiddle = byte;
        this._state = ParserState.WaitMessageIdHigh;
        return;

      case ParserState.WaitMessageIdHigh:
        if (this._version === MavlinkVersion.V1) {
          this._messageId = byte;
        } else {
          this._messageIdHigh = byte;
          this._messageId =
            (this._messageIdHigh << 16) ^
            (this._messageIdMiddle << 8) ^
            this._messageIdLow;
        }
        this._state =
          this._payloadLength === 0
            ? ParserState.WaitCrcLowByte
            : ParserState.WaitPayloadEnd;
        this._payloadCursor = 0;
        return;

      case ParserState.WaitPayloadEnd:
        if (this._payloadCursor < this._payloadLength) {
          this._payload[this._payloadCursor] = byte;
          this._payloadCursor += 1;
        }
        if (this._payloadCursor === this._payloadLength) {
          this._state = ParserState.WaitCrcLowByte;
        }
        return;

      case ParserState.WaitCrcLowByte:
        this._crcLowByte = byte;
        this._state = ParserState.WaitCrcHighByte;
        return;

      case ParserState.WaitCrcHighByte:
        this._crcHighByte = byte;
        if (
          this._version === MavlinkVersion.V2 &&
          (this._incompatibilityFlags & MavlinkParser.MAVLINK_IFLAG_SIGNED) !==
            0
        ) {
          this.onSignedPacketDropped?.(this._messageId);
          this._signatureBytesRemaining =
            MavlinkParser.MAVLINK_SIGNATURE_LENGTH;
          this._state = ParserState.WaitSignatureTrailer;
          return;
        }

        this.addMavlinkFrame();
        this.resetContext();
        this._state = ParserState.Init;
        return;

      case ParserState.WaitSignatureTrailer:
        this._signatureBytesRemaining -= 1;
        if (this._signatureBytesRemaining === 0) {
          this.resetContext();
          this._state = ParserState.Init;
        }
        return;
    }
  }

  private addMavlinkFrame(): boolean {
    if (!this.checkCrc()) {
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
