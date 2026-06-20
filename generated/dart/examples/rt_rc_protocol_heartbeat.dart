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
  final droneNode = MavlinkNode(droneSystemId, droneComponentId);

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
    watch: {droneNode},
  );

  final connectionEvents = <String>[];
  final monitorSub = gcsMonitor.onConnected.listen(
    (node) => connectionEvents.add('connected $node'),
  );
  final disconnectSub = gcsMonitor.onDisconnected.listen(
    (node) => connectionEvents.add('disconnected $node'),
  );

  gcsMonitor.start();
  gcsPublisher.start();
  dronePublisher.start();

  await Future<void>.delayed(const Duration(milliseconds: 1200));
  print('Drone online: ${gcsMonitor.isOnline(droneNode)}');
  final state = gcsMonitor.stateFor(droneNode);
  if (state != null) {
    print(
      'Drone heartbeat: type=${state.heartbeat.type} '
      'status=${state.heartbeat.systemStatus}',
    );
  }

  dronePublisher.stop();
  await Future<void>.delayed(const Duration(milliseconds: 2500));
  print('Drone online after stop: ${gcsMonitor.isOnline(droneNode)}');
  print('Events: ${connectionEvents.join(', ')}');

  await monitorSub.cancel();
  await disconnectSub.cancel();
  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
