import 'package:mavlink/mavlink_protocols.dart';

/// Ground control station identity (MAVLink convention).
const gcsSystemId = 255;
const gcsComponentId = 190;

/// Shared MAVLink session state and protocol clients for the interactive GCS.
class GcsContext {
  GcsContext({
    required this.session,
    required this.dialect,
    required this.vehicle,
    required this.heartbeatMonitor,
    required this.heartbeatPublisher,
  })  : parameters = ParameterProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          requestTimeout: const Duration(seconds: 10),
          idleTimeout: const Duration(seconds: 2),
        ),
        mission = MissionProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          itemTimeout: const Duration(seconds: 10),
          operationTimeout: const Duration(seconds: 30),
        ),
        command = CommandProtocol(
          session: session,
          targetSystem: vehicle.systemId,
          targetComponent: vehicle.componentId,
          defaultTimeout: const Duration(seconds: 10),
        );

  final MavlinkSession session;
  final MavlinkDialect dialect;
  final MavlinkNode vehicle;
  final HeartbeatMonitor heartbeatMonitor;
  final HeartbeatPublisher heartbeatPublisher;

  final ParameterProtocol parameters;
  final MissionProtocol mission;
  final CommandProtocol command;

  /// Last fetched parameter snapshot (used for read/write helpers in the CLI).
  List<ParamEntry> cachedParameters = [];

  int get targetSystem => vehicle.systemId;
  int get targetComponent => vehicle.componentId;
}
