/** Shared helpers for MAVLink JavaScript examples. */

import { MavlinkFrame } from '../mavlink_frame.js';

export * from '../mavlink.js';

export const gcsSystemId = 255;
export const gcsComponentId = 190;

export const droneSystemId = 1;
export const droneComponentId = 1;

export function frameFromGcs(message, sequence = 0) {
  return MavlinkFrame.v2(sequence, gcsSystemId, gcsComponentId, message);
}

export function frameFromDrone(message, sequence = 0) {
  return MavlinkFrame.v2(sequence, droneSystemId, droneComponentId, message);
}

export function paramIdFromString(name) {
  const paramId = [];
  for (const ch of name.slice(0, 16)) {
    paramId.push(ch.charCodeAt(0));
  }
  while (paramId.length < 16) {
    paramId.push(0);
  }
  return paramId;
}

export function paramIdToString(paramId) {
  const end = paramId.findIndex((value) => value === 0);
  const length = end === -1 ? paramId.length : end;
  return String.fromCharCode(...paramId.slice(0, length));
}

export function logFrame(direction, frame) {
  console.log(
    `${direction} msgId=${frame.message.mavlinkMessageId} ` +
      `sys=${frame.systemId} comp=${frame.componentId}`,
  );
}

export function roundTripMessage(dialect, message) {
  return dialect.parse(message.mavlinkMessageId, message.serialize());
}
