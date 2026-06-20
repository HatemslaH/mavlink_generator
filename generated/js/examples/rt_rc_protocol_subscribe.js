#!/usr/bin/env node
/** Typed message subscription example for the `rt_rc` dialect. */

import {
  Attitude,
  MavlinkNode,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common.js';

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);
  const vehicle = new MavlinkNode(droneSystemId, droneComponentId);

  const attitudeSamples = [];
  const subscription = link.gcs.listenMessage(
    Attitude,
    (message) => attitudeSamples.push(message),
    { fromSystemId: vehicle.systemId },
  );

  await link.drone.send(new Attitude(1000, 0.1, -0.05, 1.57, 0, 0, 0));

  await delay(50);
  subscription.cancel();

  console.log(`Received ${attitudeSamples.length} ATTITUDE samples via listenMessage`);
  if (attitudeSamples.length > 0) {
    const sample = attitudeSamples[0];
    console.log(`  roll=${sample.roll} pitch=${sample.pitch} yaw=${sample.yaw}`);
  }

  await closeVirtualLink({ bus: link.bus, gcs: link.gcs, drone: link.drone });
}

main();
