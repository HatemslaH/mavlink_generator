export class CrcX25 {
  private static readonly X25_INIT_CRC = 0xffff;

  private _crc: number;

  constructor() {
    this._crc = CrcX25.X25_INIT_CRC;
  }

  get crc(): number {
    return this._crc & 0xffff;
  }

  accumulate(byte: number): void {
    let b = byte & 0xff;
    let tmp = b ^ (this._crc & 0xff);
    tmp &= 0xff;
    tmp ^= (tmp << 4) & 0xff;
    this._crc =
      (this._crc >> 8) ^
      ((tmp << 8) & 0xffff) ^
      ((tmp << 3) & 0xffff) ^
      (tmp >> 4);
  }

  accumulateString(text: string): void {
    for (const codeUnit of text) {
      this.accumulate(codeUnit.charCodeAt(0) & 0xff);
    }
  }
}
