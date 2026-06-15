// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual mission upload for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
void main() {
  final dialect = MavlinkDialectRt_rc();
  const missionType = MavMissionType.mavMissionTypeMission;

  final missionItems = <MissionItem>[
    MissionItem(
      param1: 0,
      param2: 2,
      param3: 0,
      param4: 0,
      x: 47.397742,
      y: 8.545594,
      z: 50,
      seq: 0,
      command: MavCmd.mavCmdNavWaypoint,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
      frame: MavFrame.mavFrameGlobalRelativeAlt,
      current: 0,
      autocontinue: 1,
      missionType: missionType,
    ),
    MissionItem(
      param1: 0,
      param2: 2,
      param3: 0,
      param4: 0,
      x: 47.398000,
      y: 8.546000,
      z: 50,
      seq: 1,
      command: MavCmd.mavCmdNavWaypoint,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
      frame: MavFrame.mavFrameGlobalRelativeAlt,
      current: 0,
      autocontinue: 1,
      missionType: missionType,
    ),
  ];

  var seq = 0;

  // 1. GCS announces mission size.
  final count = MissionCount(
    count: missionItems.length,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    missionType: missionType,
  );
  final countFrame = frameFromGcs(count, sequence: 1);
  logFrame('GCS ->', countFrame);
  roundTripMessage(dialect, count);

  // 2. Drone requests each mission item, GCS responds.
  while (seq < missionItems.length) {
    final request = MissionRequest(
      seq: seq,
      targetSystem: gcsSystemId,
      targetComponent: gcsComponentId,
      missionType: missionType,
    );
    final requestFrame = frameFromDrone(request, sequence: seq + 10);
    logFrame('Drone ->', requestFrame);
    roundTripMessage(dialect, request);

    final item = missionItems[seq];
    final itemFrame = frameFromGcs(item, sequence: seq + 20);
    logFrame('GCS ->', itemFrame);
    final parsedItem = roundTripMessage(dialect, item);
    if (parsedItem is MissionItem) {
      print('  uploaded seq=${parsedItem.seq} cmd=${parsedItem.command}');
    }

    seq++;
  }

  // 3. Drone accepts the mission.
  final ack = MissionAck(
    targetSystem: gcsSystemId,
    targetComponent: gcsComponentId,
    type: MavMissionResult.mavMissionAccepted,
    missionType: missionType,
  );
  final ackFrame = frameFromDrone(ack, sequence: 99);
  logFrame('Drone ->', ackFrame);
  final parsedAck = roundTripMessage(dialect, ack);
  if (parsedAck is MissionAck) {
    print('Mission upload complete: ${parsedAck.type}');
  }
}
