import {
  MavCmd,
  type MissionItemInt,
} from '../../../generated/ts/dialects/rt_rc.ts';
import { MissionItems } from '../../../generated/ts/protocols/mission_protocol.ts';

/** Hardcoded sample mission (Zurich area coordinates, same as generated virtual examples). */
export function buildSampleMission(options: {
  targetSystem: number;
  targetComponent: number;
}): MissionItemInt[] {
  return MissionItems.withSequentialSeq([
    MissionItems.waypoint({
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: options.targetSystem,
      targetComponent: options.targetComponent,
    }),
    MissionItems.waypoint({
      seq: 1,
      latitude: 47.398,
      longitude: 8.546,
      altitude: 50,
      targetSystem: options.targetSystem,
      targetComponent: options.targetComponent,
    }),
    MissionItems.waypoint({
      seq: 2,
      latitude: 47.398258,
      longitude: 8.546406,
      altitude: 50,
      targetSystem: options.targetSystem,
      targetComponent: options.targetComponent,
      command: MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
    }),
  ]);
}

export function describeMissionItem(item: MissionItemInt): string {
  const lat = item.x / 1e7;
  const lon = item.y / 1e7;
  const commandName = MavCmd[item.command] ?? String(item.command);
  return `seq=${item.seq} ${commandName} lat=${lat.toFixed(6)} lon=${lon.toFixed(6)} alt=${item.z}m`;
}
