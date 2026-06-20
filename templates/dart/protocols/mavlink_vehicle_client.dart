import '../mavlink_dialect.dart';
import 'command_protocol.dart';
import 'heartbeat_protocol.dart';
import 'mavlink_link.dart';
import 'mavlink_session.dart';
import 'mission_protocol.dart';
import 'parameter_protocol.dart';

/// Protocol clients bound to a single remote MAVLink vehicle.
///
/// Created after a vehicle is discovered (e.g. via [HeartbeatMonitor.waitForVehicle]).
/// Shares one [MavlinkSession] and consistent target addressing for all services.
class MavlinkVehicleClient {
  MavlinkVehicleClient({
    required this.session,
    required this.vehicle,
    Duration parameterRequestTimeout = const Duration(seconds: 10),
    Duration parameterIdleTimeout = const Duration(seconds: 2),
    Duration missionItemTimeout = const Duration(seconds: 10),
    Duration missionOperationTimeout = const Duration(seconds: 30),
    Duration commandTimeout = const Duration(seconds: 10),
  })  : parameters = ParameterProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          requestTimeout: parameterRequestTimeout,
          idleTimeout: parameterIdleTimeout,
        ),
        mission = MissionProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          itemTimeout: missionItemTimeout,
          operationTimeout: missionOperationTimeout,
        ),
        command = CommandProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          defaultTimeout: commandTimeout,
        );

  final MavlinkSession session;
  final MavlinkNode vehicle;

  final ParameterProtocol parameters;
  final MissionProtocol mission;
  final CommandProtocol command;

  int get targetSystem => vehicle.systemId;
  int get targetComponent => vehicle.componentId;
}

/// Ground control station bootstrap: session, heartbeat publisher, and monitor.
///
/// Use [waitForVehicle] then [vehicleClient] to obtain protocol clients.
class MavlinkGcs {
  MavlinkGcs({
    required this.session,
    required this.heartbeatPublisher,
    required this.heartbeatMonitor,
  });

  final MavlinkSession session;
  final HeartbeatPublisher heartbeatPublisher;
  final HeartbeatMonitor heartbeatMonitor;

  /// Start heartbeat publish/monitor loops.
  void start() {
    heartbeatMonitor.start();
    heartbeatPublisher.start();
  }

  /// Stop heartbeat publish/monitor loops.
  Future<void> stopHeartbeats() async {
    heartbeatPublisher.stop();
    await heartbeatMonitor.stop();
  }

  /// Wait for the first vehicle and return a [MavlinkVehicleClient] for it.
  Future<MavlinkVehicleClient> waitForVehicle({
    Set<int>? excludeSystemIds,
    Duration timeout = const Duration(seconds: 60),
  }) {
    return heartbeatMonitor
        .waitForVehicle(excludeSystemIds: excludeSystemIds, timeout: timeout)
        .then((node) => MavlinkVehicleClient(session: session, vehicle: node));
  }

  /// Build a [MavlinkVehicleClient] for a known [vehicle] node.
  MavlinkVehicleClient vehicleClient(MavlinkNode vehicle) {
    return MavlinkVehicleClient(session: session, vehicle: vehicle);
  }

  /// Factory for a typical GCS setup over [link].
  factory MavlinkGcs.connect({
    required MavlinkDialect dialect,
    required MavlinkLink link,
    int systemId = 255,
    int componentId = 190,
    Duration heartbeatInterval = const Duration(seconds: 1),
    Duration heartbeatTimeout = const Duration(seconds: 3),
  }) {
    final session = MavlinkSession(
      dialect: dialect,
      link: link,
      systemId: systemId,
      componentId: componentId,
    );

    return MavlinkGcs(
      session: session,
      heartbeatPublisher: HeartbeatPublisher(
        session: session,
        heartbeat: HeartbeatTemplates.gcs(mavlinkVersion: dialect.version),
        interval: heartbeatInterval,
      ),
      heartbeatMonitor: HeartbeatMonitor(session: session, timeout: heartbeatTimeout),
    );
  }

  Future<void> close() async {
    await stopHeartbeats();
    await session.close();
  }
}
