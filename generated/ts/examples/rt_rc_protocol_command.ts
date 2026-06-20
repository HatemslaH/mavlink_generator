/** Command protocol example for the `rt_rc` dialect. */

import {
  Attitude,
  CommandProtocol,
  CommandServer,
  MavResult,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common';

async function main(): Promise<void> {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);

  const commandServer = new CommandServer({
    session: link.drone,
    onCommandLong: async (command) => {
      console.log(
        `Vehicle received COMMAND_LONG: ${command.command} ` +
          `p1=${command.param1} p2=${command.param2}`,
      );
      return MavResult.MAV_RESULT_ACCEPTED;
    },
  });

  const commandProtocol = new CommandProtocol({
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  });

  const intervalAck = await commandProtocol.setMessageInterval(Attitude.MSG_ID, 100000);
  console.log(`SET_MESSAGE_INTERVAL ack: ${intervalAck.result}`);

  const requestAck = await commandProtocol.requestMessage(Attitude.MSG_ID);
  console.log(`REQUEST_MESSAGE ack: ${requestAck.result}`);

  const armAck = await commandProtocol.arm();
  console.log(`ARM ack: ${armAck.result}`);

  const disarmAck = await commandProtocol.disarm();
  console.log(`DISARM ack: ${disarmAck.result}`);

  await commandServer.close();
  await closeVirtualLink(link);
}

void main();
