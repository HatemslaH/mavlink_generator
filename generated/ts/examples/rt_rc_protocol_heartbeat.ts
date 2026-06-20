/** Heartbeat protocol example for the `rt_rc` dialect. */

import {
  HeartbeatMonitor,
  HeartbeatPublisher,
  HeartbeatTemplates,
  closeVirtualLink,
  createVirtualLink,
  gcsSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common';

async function main(): Promise<void> {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);

  const gcsPublisher = new HeartbeatPublisher({
    session: link.gcs,
    heartbeat: HeartbeatTemplates.gcs(dialect.version),
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
  console.log(`Vehicle discovered: ${vehicle.toString()}`);
  console.log(`Drone online: ${gcsMonitor.isOnline(vehicle)}`);
  const state = gcsMonitor.stateFor(vehicle);
  if (state !== null) {
    console.log(
      `Drone heartbeat: type=${state.heartbeat.type} status=${state.heartbeat.system_status}`,
    );
  }

  dronePublisher.stop();
  await new Promise((resolve) => setTimeout(resolve, 2500));
  console.log(`Drone online after stop: ${gcsMonitor.isOnline(vehicle)}`);

  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink(link);
}

void main();
