/** Typed message subscription example for the `rt_rc` dialect. */

import {
  Attitude,
  MavlinkNode,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common';

async function main(): Promise<void> {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);
  const vehicle = new MavlinkNode(droneSystemId, droneComponentId);

  const attitudeSamples: Attitude[] = [];
  const subscription = link.gcs.listenMessage(
    (message) => {
      attitudeSamples.push(message);
    },
    {
      fromSystemId: vehicle.systemId,
      messageType: Attitude,
    },
  );

  await link.drone.send(new Attitude(1000, 0.1, -0.05, 1.57, 0, 0, 0));

  await new Promise((resolve) => setTimeout(resolve, 50));
  subscription.cancel();

  console.log(`Received ${attitudeSamples.length} ATTITUDE samples via listenMessage`);
  if (attitudeSamples.length > 0) {
    const sample = attitudeSamples[0]!;
    console.log(`  roll=${sample.roll} pitch=${sample.pitch} yaw=${sample.yaw}`);
  }

  await closeVirtualLink(link);
}

void main();
