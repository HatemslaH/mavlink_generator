// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Mission protocol example for the `rt_rc` dialect.
///
/// Uses [MissionProtocol] on the GCS side and [MissionServer] on the vehicle
/// side over a transport-agnostic in-memory [VirtualMavlinkBus].
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final link = createVirtualLink(dialect);

  final missionServer = MissionServer(session: link.drone);
  final commandServer = CommandServer(session: link.drone);
  final missionProtocol = MissionProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final plan = <MissionItemInt>[
    MissionItems.waypoint(
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    ),
    MissionItems.waypoint(
      seq: 1,
      latitude: 47.398000,
      longitude: 8.546000,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    ),
  ];

  final uploadResult = await missionProtocol.upload(
    plan,
    onProgress: (sent, total, item) {
      print('Upload progress $sent/$total seq=${item.seq} cmd=${item.command}');
    },
  );
  print('Mission upload result: $uploadResult');
  print('Vehicle stored ${missionServer.items.length} items');

  final downloaded = await missionProtocol.download(
    onProgress: (received, total, item) {
      print('Download progress $received/$total seq=${item.seq}');
    },
  );
  print('Downloaded ${downloaded.length} mission items');

  final commandProtocol = CommandProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );
  final setCurrent = await missionProtocol.setCurrentWithCommand(
    0,
    command: commandProtocol,
  );
  print('Set current seq=${setCurrent.sequence} ack=${setCurrent.commandAck?.result}');

  final clearResult = await missionProtocol.clear();
  print('Mission clear result: $clearResult');

  await missionServer.close();
  await commandServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
