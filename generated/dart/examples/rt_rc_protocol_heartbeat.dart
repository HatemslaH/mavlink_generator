// ignore_for_file: avoid_print

import 'dart:async';

import 'protocols_common.dart';

/// Heartbeat protocol example for the `rt_rc` dialect.
///
/// Uses [HeartbeatPublisher] to send heartbeats and [HeartbeatMonitor] to track
/// remote node connectivity over a transport-agnostic link.
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final link = createVirtualLink(dialect);

  final gcsPublisher = HeartbeatPublisher(
    session: link.gcs,
    heartbeat: HeartbeatTemplates.gcs(mavlinkVersion: dialect.version),
    interval: const Duration(milliseconds: 500),
  );

  final dronePublisher = HeartbeatPublisher(
    session: link.drone,
    heartbeat: HeartbeatTemplates.autopilot(mavlinkVersion: dialect.version),
    interval: const Duration(milliseconds: 500),
  );

  final gcsMonitor = HeartbeatMonitor(
    session: link.gcs,
    timeout: const Duration(seconds: 2),
  );

  gcsMonitor.start();
  gcsPublisher.start();
  dronePublisher.start();

  final vehicle = await gcsMonitor.waitForVehicle(
    excludeSystemIds: {gcsSystemId},
    timeout: const Duration(seconds: 5),
  );
  print('Vehicle discovered: $vehicle');
  print('Drone online: ${gcsMonitor.isOnline(vehicle)}');
  final state = gcsMonitor.stateFor(vehicle);
  if (state != null) {
    print(
      'Drone heartbeat: type=${state.heartbeat.type} '
      'status=${state.heartbeat.systemStatus}',
    );
  }

  dronePublisher.stop();
  await Future<void>.delayed(const Duration(milliseconds: 2500));
  print('Drone online after stop: ${gcsMonitor.isOnline(vehicle)}');

  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
