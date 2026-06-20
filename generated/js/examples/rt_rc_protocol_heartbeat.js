#!/usr/bin/env node
/** Heartbeat protocol example for the `rt_rc` dialect. */

import {
  HeartbeatMonitor,
  HeartbeatPublisher,
  HeartbeatTemplates,
  closeVirtualLink,
  createVirtualLink,
  gcsSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common.js';

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);

  const gcsPublisher = new HeartbeatPublisher({
    session: link.gcs,
    heartbeat: HeartbeatTemplates.gcs({ mavlinkVersion: dialect.version }),
    intervalMs: 500,
  });

  const dronePublisher = new HeartbeatPublisher({
    session: link.drone,
    heartbeat: HeartbeatTemplates.autopilot({ mavlinkVersion: dialect.version }),
    intervalMs: 500,
  });

  const gcsMonitor = new HeartbeatMonitor({
    session: link.gcs,
    timeoutMs: 2000,
  });

  gcsMonitor.start();
  gcsPublisher.start();
  dronePublisher.start();

  const vehicle = await gcsMonitor.waitForVehicle({
    excludeSystemIds: new Set([gcsSystemId]),
    timeoutMs: 5000,
  });
  console.log(`Vehicle discovered: ${vehicle}`);
  console.log(`Drone online: ${gcsMonitor.isOnline(vehicle)}`);
  const state = gcsMonitor.stateFor(vehicle);
  if (state != null) {
    console.log(
      `Drone heartbeat: type=${state.heartbeat.type} status=${state.heartbeat.system_status}`,
    );
  }

  dronePublisher.stop();
  await delay(2500);
  console.log(`Drone online after stop: ${gcsMonitor.isOnline(vehicle)}`);

  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink({ bus: link.bus, gcs: link.gcs, drone: link.drone });
}

main();
