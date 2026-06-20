export * from '../mavlink_protocols.js';
import {
  MavlinkSession,
  VirtualMavlinkBus,
} from '../mavlink_protocols.js';

/** Ground control station identity (MAVLink convention). */
export const gcsSystemId = 255;
export const gcsComponentId = 190;

/** Simulated autopilot identity. */
export const droneSystemId = 1;
export const droneComponentId = 1;

/** Create a linked GCS/drone pair over an in-memory MAVLink bus. */
export function createVirtualLink(dialect) {
  const bus = new VirtualMavlinkBus();
  const gcsLink = bus.createEndpoint();
  const droneLink = bus.createEndpoint();

  const gcs = new MavlinkSession({
    dialect,
    link: gcsLink,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  });

  const drone = new MavlinkSession({
    dialect,
    link: droneLink,
    systemId: droneSystemId,
    componentId: droneComponentId,
  });

  return { bus, gcs, drone, dialect };
}

export async function closeVirtualLink({ bus, gcs, drone }) {
  await gcs.close();
  await drone.close();
  await bus.closeAll();
}
