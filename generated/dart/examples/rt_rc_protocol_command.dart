// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Command protocol example for the `rt_rc` dialect.
///
/// Uses [CommandProtocol] on the GCS side and [CommandServer] on the vehicle
/// side. Demonstrates message interval setup and one-shot telemetry requests.
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final link = createVirtualLink(dialect);

  final commandServer = CommandServer(
    session: link.drone,
    onCommandLong: (command) async {
      print(
        'Vehicle received COMMAND_LONG: ${command.command} '
        'p1=${command.param1} p2=${command.param2}',
      );
      return MavResult.mavResultAccepted;
    },
  );

  final commandProtocol = CommandProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final intervalAck = await commandProtocol.setMessageInterval(
    Attitude.msgId,
    100000,
  );
  print('SET_MESSAGE_INTERVAL ack: ${intervalAck.result}');

  final requestAck = await commandProtocol.requestMessage(Attitude.msgId);
  print('REQUEST_MESSAGE ack: ${requestAck.result}');

  await commandServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
