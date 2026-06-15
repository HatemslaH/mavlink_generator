/** Shared helpers for MAVLink TypeScript examples. */

export * from '../mavlink';
import { MavlinkDialect, MavlinkFrame, MavlinkMessage } from '../mavlink';

/** Ground control station identity (MAVLink convention). */
export const gcsSystemId = 255;
export const gcsComponentId = 190;

/** Simulated autopilot identity. */
export const droneSystemId = 1;
export const droneComponentId = 1;

export function frameFromGcs(
  message: MavlinkMessage,
  sequence = 0,
): MavlinkFrame {
  return MavlinkFrame.v2(sequence, gcsSystemId, gcsComponentId, message);
}

export function frameFromDrone(
  message: MavlinkMessage,
  sequence = 0,
): MavlinkFrame {
  return MavlinkFrame.v2(sequence, droneSystemId, droneComponentId, message);
}

export function paramIdFromString(name: string): number[] {
  const paramId: number[] = [];
  for (const codeUnit of name.slice(0, 16)) {
    paramId.push(codeUnit.charCodeAt(0));
  }
  while (paramId.length < 16) {
    paramId.push(0);
  }
  return paramId;
}

export function paramIdToString(paramId: number[]): string {
  const end = paramId.findIndex((value) => value === 0);
  const slice = end === -1 ? paramId : paramId.slice(0, end);
  return String.fromCharCode(...slice);
}

export function logFrame(direction: string, frame: MavlinkFrame): void {
  console.log(
    `${direction} msgId=${frame.message.mavlinkMessageId} ` +
      `sys=${frame.systemId} comp=${frame.componentId}`,
  );
}

export function roundTripMessage(
  dialect: MavlinkDialect,
  message: MavlinkMessage,
): MavlinkMessage | null {
  return dialect.parse(message.mavlinkMessageId, message.serialize());
}
