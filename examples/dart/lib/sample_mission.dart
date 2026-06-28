import 'package:mavlink/mavlink_protocols.dart';

/// Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples).
List<MissionItemInt> buildSampleMission({
  required int targetSystem,
  required int targetComponent,
}) {
  return MissionItems.withSequentialSeq([
    MissionItems.waypoint(
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: targetSystem,
      targetComponent: targetComponent,
    ),
    MissionItems.waypoint(
      seq: 1,
      latitude: 47.398000,
      longitude: 8.546000,
      altitude: 50,
      targetSystem: targetSystem,
      targetComponent: targetComponent,
    ),
    MissionItems.waypoint(
      seq: 2,
      latitude: 47.398258,
      longitude: 8.546406,
      altitude: 50,
      targetSystem: targetSystem,
      targetComponent: targetComponent,
      command: MavCmd.mavCmdNavReturnToLaunch,
    ),
  ]);
}

String describeMissionItem(MissionItemInt item) {
  final lat = item.x / 1e7;
  final lon = item.y / 1e7;
  return 'seq=${item.seq} ${item.command.name} '
      'lat=${lat.toStringAsFixed(6)} lon=${lon.toStringAsFixed(6)} alt=${item.z}m';
}
