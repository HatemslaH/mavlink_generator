export * from '../mavlink_protocols';
import type { MavlinkDialect } from '../mavlink_dialect';
import {
  MavlinkSession,
  VirtualMavlinkBus,
} from '../mavlink_protocols';

/** Ground control station identity (MAVLink convention). */
export const gcsSystemId = 255;
export const gcsComponentId = 190;

/** Simulated autopilot identity. */
export const droneSystemId = 1;
export const droneComponentId = 1;

export interface VirtualLink {
  readonly bus: VirtualMavlinkBus;
  readonly gcs: MavlinkSession;
  readonly drone: MavlinkSession;
  readonly dialect: MavlinkDialect;
}

/** Create a linked GCS/drone pair over an in-memory MAVLink bus. */
export function createVirtualLink(dialect: MavlinkDialect): VirtualLink {
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

/** Close a virtual link created by [createVirtualLink]. */
export async function closeVirtualLink(link: {
  bus: VirtualMavlinkBus;
  gcs: MavlinkSession;
  drone: MavlinkSession;
}): Promise<void> {
  await link.gcs.close();
  await link.drone.close();
  await link.bus.closeAll();
}
