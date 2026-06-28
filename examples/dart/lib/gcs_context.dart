import 'package:mavlink/mavlink_protocols.dart';

/// Ground control station identity (MAVLink convention).
const gcsSystemId = 255;
const gcsComponentId = 190;

/// Shared MAVLink GCS state for the interactive SITL example.
class GcsContext {
  GcsContext({
    required this.gcs,
    required this.vehicle,
    required this.client,
  });

  final MavlinkGcs gcs;
  final MavlinkNode vehicle;
  final MavlinkVehicleClient client;

  /// Cancels in-flight parameter/mission operations (type `cancel` in CLI).
  MavlinkCancellationToken? operationCancel;

  MavlinkSession get session => gcs.session;
  HeartbeatMonitor get heartbeatMonitor => gcs.heartbeatMonitor;
  HeartbeatPublisher get heartbeatPublisher => gcs.heartbeatPublisher;

  ParameterProtocol get parameters => client.parameters;
  MissionProtocol get mission => client.mission;
  CommandProtocol get command => client.command;

  int get targetSystem => vehicle.systemId;
  int get targetComponent => vehicle.componentId;
}
