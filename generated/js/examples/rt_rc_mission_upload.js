#!/usr/bin/env node
/** Virtual mission upload for the `rt_rc` dialect. */

import {
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  gcsComponentId,
  gcsSystemId,
  logFrame,
  roundTripMessage,
  MissionAck,
  MissionCount,
  MissionItem,
  MissionRequest,
  MavCmd,
  MavFrame,
  MavMissionResult,
  MavMissionType,
  MavlinkDialectRt_rc,
} from './common.js';

function main() {
  const dialect = new MavlinkDialectRt_rc();
  const missionType = MavMissionType.MAV_MISSION_TYPE_MISSION;

  const missionItems = [
    new MissionItem(
      0, 2, 0, 0,
      47.397742, 8.545594, 50,
      0,
      MavCmd.MAV_CMD_NAV_WAYPOINT,
      droneSystemId, droneComponentId,
      MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1, missionType,
    ),
    new MissionItem(
      0, 2, 0, 0,
      47.398000, 8.546000, 50,
      1,
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
  logFrame('GCS ->', frameFromGcs(count, 1));
  roundTripMessage(dialect, count);

  while (seq < missionItems.length) {
    const request = new MissionRequest(
      seq,
      gcsSystemId,
      gcsComponentId,
      missionType,
    );
    logFrame('Drone ->', frameFromDrone(request, seq + 10));
    roundTripMessage(dialect, request);

    const item = missionItems[seq];
    logFrame('GCS ->', frameFromGcs(item, seq + 20));
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
  logFrame('Drone ->', frameFromDrone(ack, 99));
  const parsedAck = roundTripMessage(dialect, ack);
  if (parsedAck instanceof MissionAck) {
    console.log(`Mission upload complete: ${parsedAck.type}`);
  }
}

main();
