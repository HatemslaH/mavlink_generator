#!/usr/bin/env node
/** MavlinkGcs / MavlinkVehicleClient facade example for `rt_rc`. */

import {
  CommandServer,
  Heartbeat,
  HeartbeatPublisher,
  HeartbeatTemplates,
  MavParamType,
  MavlinkGcs,
  MavlinkSession,
  ParameterServer,
  VirtualMavlinkBus,
  droneComponentId,
  droneSystemId,
  gcsComponentId,
  gcsSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common.js';

async function main() {
  const dialect = new MavlinkDialectRt_rc();
  const bus = new VirtualMavlinkBus();
  const gcsLink = bus.createEndpoint();
  const droneLink = bus.createEndpoint();

  const gcs = MavlinkGcs.connect({
    dialect,
    link: gcsLink,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  });

  const droneSession = new MavlinkSession({
    dialect,
    link: droneLink,
    systemId: droneSystemId,
    componentId: droneComponentId,
  });

  const dronePublisher = new HeartbeatPublisher({
    session: droneSession,
    heartbeat: HeartbeatTemplates.autopilot({ mavlinkVersion: dialect.version }),
    intervalMs: 500,
  });

  const parameterServer = new ParameterServer({
    session: droneSession,
    initialValues: {
      SYSID_THISMAV: { value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 },
    },
  });

  const commandServer = new CommandServer({ session: droneSession });

  gcs.start();
  dronePublisher.start();

  const client = await gcs.waitForVehicle({ excludeSystemIds: new Set([gcsSystemId]) });
  console.log(`Connected to vehicle ${client.vehicle}`);

  const params = await client.parameters.fetchAll();
  console.log(`Vehicle has ${params.length} parameters`);

  const ack = await client.command.requestMessage(Heartbeat.MSG_ID);
  console.log(`REQUEST_MESSAGE ack: ${ack.result}`);

  await parameterServer.close();
  await commandServer.close();
  dronePublisher.stop();
  await droneSession.close();
  await gcs.close();
  await bus.closeAll();
}

main();
