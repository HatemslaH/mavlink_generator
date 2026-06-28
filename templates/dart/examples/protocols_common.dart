export '../lib/mavlink_protocols.dart';
import '../lib/mavlink_protocols.dart';

/// Ground control station identity (MAVLink convention).
const gcsSystemId = 255;
const gcsComponentId = 190;

/// Simulated autopilot identity.
const droneSystemId = 1;
const droneComponentId = 1;

/// Create a linked GCS/drone pair over an in-memory MAVLink bus.
({
  VirtualMavlinkBus bus,
  MavlinkSession gcs,
  MavlinkSession drone,
  MavlinkDialect dialect,
}) createVirtualLink(MavlinkDialect dialect) {
  final bus = VirtualMavlinkBus();
  final gcsLink = bus.createEndpoint();
  final droneLink = bus.createEndpoint();

  final gcs = MavlinkSession(
    dialect: dialect,
    link: gcsLink,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  );

  final drone = MavlinkSession(
    dialect: dialect,
    link: droneLink,
    systemId: droneSystemId,
    componentId: droneComponentId,
  );

  return (bus: bus, gcs: gcs, drone: drone, dialect: dialect);
}

Future<void> closeVirtualLink({
  required VirtualMavlinkBus bus,
  required MavlinkSession gcs,
  required MavlinkSession drone,
}) async {
  await gcs.close();
  await drone.close();
  await bus.closeAll();
}
