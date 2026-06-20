// ignore_for_file: avoid_print

import 'dart:async';

import 'protocols_common.dart';

/// Typed message subscription example for the `rt_rc` dialect.
///
/// Uses [MavlinkSession.onMessage] and [MavlinkSession.listenMessage] to receive
/// telemetry without manual request/wait loops.
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final link = createVirtualLink(dialect);
  final vehicle = MavlinkNode(droneSystemId, droneComponentId);

  final attitudeSamples = <Attitude>[];
  final subscription = link.gcs.listenMessage<Attitude>(
    (message, frame) => attitudeSamples.add(message),
    fromSystemId: vehicle.systemId,
  );

  await link.drone.send(
    Attitude(
      timeBootMs: 1000,
      roll: 0.1,
      pitch: -0.05,
      yaw: 1.57,
      rollspeed: 0,
      pitchspeed: 0,
      yawspeed: 0,
    ),
  );

  await Future<void>.delayed(const Duration(milliseconds: 50));
  subscription.cancel();

  print('Received ${attitudeSamples.length} ATTITUDE samples via listenMessage');
  if (attitudeSamples.isNotEmpty) {
    final sample = attitudeSamples.first;
    print('  roll=${sample.roll} pitch=${sample.pitch} yaw=${sample.yaw}');
  }

  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
