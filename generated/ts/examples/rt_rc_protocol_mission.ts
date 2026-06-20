/** Mission protocol example for the `rt_rc` dialect over VirtualMavlinkBus. */

import {
  CommandProtocol,
  CommandServer,
  MissionItems,
  MissionProtocol,
  MissionServer,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common';

async function main(): Promise<void> {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);

  const missionServer = new MissionServer({ session: link.drone });
  const commandServer = new CommandServer({ session: link.drone });
  const missionProtocol = new MissionProtocol({
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  });

  const plan = [
    MissionItems.waypoint({
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    }),
    MissionItems.waypoint({
      seq: 1,
      latitude: 47.398,
      longitude: 8.546,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    }),
  ];

  const uploadResult = await missionProtocol.upload(plan, {
    onProgress: (sent, total, item) => {
      console.log(`Upload progress ${sent}/${total} seq=${item.seq} cmd=${item.command}`);
    },
  });
  console.log(`Mission upload result: ${uploadResult}`);
  console.log(`Vehicle stored ${missionServer.items.length} items`);

  const downloaded = await missionProtocol.download({
    onProgress: (received, total, item) => {
      console.log(`Download progress ${received}/${total} seq=${item.seq}`);
    },
  });
  console.log(`Downloaded ${downloaded.length} mission items`);

  const commandProtocol = new CommandProtocol({
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  });
  const setCurrent = await missionProtocol.setCurrentWithCommand(0, {
    command: commandProtocol,
  });
  console.log(`Set current seq=${setCurrent.sequence} ack=${setCurrent.commandAck?.result}`);

  const clearResult = await missionProtocol.clear();
  console.log(`Mission clear result: ${clearResult}`);

  await missionServer.close();
  await commandServer.close();
  await closeVirtualLink(link);
}

void main();
