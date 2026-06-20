import { MavCmd, MissionItems } from '../../../generated/js/mavlink_protocols.js';

/** Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples). */
export function buildSampleMission({ targetSystem, targetComponent }) {
  return MissionItems.withSequentialSeq([
    MissionItems.waypoint({
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem,
      targetComponent,
    }),
    MissionItems.waypoint({
      seq: 1,
      latitude: 47.398,
      longitude: 8.546,
      altitude: 50,
      targetSystem,
      targetComponent,
    }),
    MissionItems.waypoint({
      seq: 2,
      latitude: 47.398258,
      longitude: 8.546406,
      altitude: 50,
      targetSystem,
      targetComponent,
      command: MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
    }),
  ]);
}

export function describeMissionItem(item) {
  const lat = item.x / 1e7;
  const lon = item.y / 1e7;
  return `seq=${item.seq} cmd=${item.command} lat=${lat.toFixed(6)} lon=${lon.toFixed(6)} alt=${item.z}m`;
}
