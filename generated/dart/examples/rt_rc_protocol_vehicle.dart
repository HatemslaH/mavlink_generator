// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// [MavlinkGcs] / [MavlinkVehicleClient] facade example for `rt_rc`.
///
/// Demonstrates GCS bootstrap, [HeartbeatMonitor.waitForVehicle], and a single
/// vehicle client that bundles parameter, mission, and command protocols.
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final bus = VirtualMavlinkBus();
  final gcsLink = bus.createEndpoint();
  final droneLink = bus.createEndpoint();

  final gcs = MavlinkGcs.connect(
    dialect: dialect,
    link: gcsLink,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  );

  final droneSession = MavlinkSession(
    dialect: dialect,
    link: droneLink,
    systemId: droneSystemId,
    componentId: droneComponentId,
  );

  final dronePublisher = HeartbeatPublisher(
    session: droneSession,
    heartbeat: HeartbeatTemplates.autopilot(mavlinkVersion: dialect.version),
    interval: const Duration(milliseconds: 500),
  );

  final parameterServer = ParameterServer(
    session: droneSession,
    initialValues: {
      'SYSID_THISMAV': (value: 1, type: MavParamType.mavParamTypeInt32),
    },
  );

  final commandServer = CommandServer(session: droneSession);

  gcs.start();
  dronePublisher.start();

  final client = await gcs.waitForVehicle(excludeSystemIds: {gcsSystemId});
  print('Connected to vehicle ${client.vehicle}');

  final params = await client.parameters.fetchAll();
  print('Vehicle has ${params.length} parameters');

  final ack = await client.command.requestMessage(Heartbeat.msgId);
  print('REQUEST_MESSAGE ack: ${ack.result}');

  await parameterServer.close();
  await commandServer.close();
  dronePublisher.stop();
  await droneSession.close();
  await gcs.close();
  await bus.closeAll();
}
