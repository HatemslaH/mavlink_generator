import type { MavlinkMessage } from './mavlink_message';

export interface MavlinkDialect {
  readonly version: number;
  parse(messageId: number, data: Uint8Array): MavlinkMessage | null;
  /** Return CRC extra for messageId, or -1 if unsupported. */
  crcExtra(messageId: number): number;
}
