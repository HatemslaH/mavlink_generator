/**
 * Virtual mission upload for the `rt_rc` dialect.
 *
 * Follows https://mavlink.io/en/services/mission.html upload sequence:
 * GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
 */

import {
  MavCmd,
  MavFrame,
  MavMissionResult,
  MavMissionType,
  MissionAck,
  MissionCount,
  MissionItem,
  MissionRequest,
  MavlinkDialectRt_rc,
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  gcsComponentId,
  gcsSystemId,
  logFrame,
  roundTripMessage,
} from './common';

function main(): void {
  const dialect = new MavlinkDialectRt_rc();
  const missionType = MavMissionType.MAV_MISSION_TYPE_MISSION;

  const missionItems = [
    new MissionItem(
      0, 2, 0, 0, 47.397742, 8.545594, 50, 0,
      MavCmd.MAV_CMD_NAV_WAYPOINT,
      droneSystemId, droneComponentId,
      MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1, missionType,
    ),
    new MissionItem(
      0, 2, 0, 0, 47.398, 8.546, 50, 1,
      MavCmd.MAV_CMD_NAV_WAYPOINT,
      droneSystemId, droneComponentId,
      MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1, missionType,
    ),
  ];

  let seq = 0;

  const count = new MissionCount(
    missionItems.length,
    droneSystemId,
    droneComponentId,
    missionType,
  );
  const countFrame = frameFromGcs(count, 1);
  logFrame('GCS ->', countFrame);
  roundTripMessage(dialect, count);

  while (seq < missionItems.length) {
    const request = new MissionRequest(
      seq,
      gcsSystemId,
      gcsComponentId,
      missionType,
    );
    const requestFrame = frameFromDrone(request, seq + 10);
    logFrame('Drone ->', requestFrame);
    roundTripMessage(dialect, request);

    const item = missionItems[seq]!;
    const itemFrame = frameFromGcs(item, seq + 20);
    logFrame('GCS ->', itemFrame);
    const parsedItem = roundTripMessage(dialect, item);
    if (parsedItem instanceof MissionItem) {
      console.log(`  uploaded seq=${parsedItem.seq} cmd=${parsedItem.command}`);
    }

    seq += 1;
  }

  const ack = new MissionAck(
    gcsSystemId,
    gcsComponentId,
    MavMissionResult.MAV_MISSION_ACCEPTED,
    missionType,
  );
  const ackFrame = frameFromDrone(ack, 99);
  logFrame('Drone ->', ackFrame);
  const parsedAck = roundTripMessage(dialect, ack);
  if (parsedAck instanceof MissionAck) {
    console.log(`Mission upload complete: ${parsedAck.type}`);
  }
}

main();
