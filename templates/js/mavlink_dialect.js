export class MavlinkDialect {
  get version() {
    throw new Error('not implemented');
  }

  parse(_messageId, _data) {
    throw new Error('not implemented');
  }

  crcExtra(_messageId) {
    throw new Error('not implemented');
  }
}
