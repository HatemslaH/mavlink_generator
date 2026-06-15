#!/usr/bin/env python3
"""Virtual mission upload for the `rt_rc` dialect.

Follows https://mavlink.io/en/services/mission.html upload sequence:
GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
"""

from common import *


def main() -> None:
    dialect = MavlinkDialectRt_rc()
    mission_type = MavMissionType.MAV_MISSION_TYPE_MISSION

    mission_items = [
        MissionItem(
            param1=0,
            param2=2,
            param3=0,
            param4=0,
            x=47.397742,
            y=8.545594,
            z=50,
            seq=0,
            command=MavCmd.MAV_CMD_NAV_WAYPOINT,
            target_system=drone_system_id,
            target_component=drone_component_id,
            frame=MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current=0,
            autocontinue=1,
            mission_type=mission_type,
        ),
        MissionItem(
            param1=0,
            param2=2,
            param3=0,
            param4=0,
            x=47.398000,
            y=8.546000,
            z=50,
            seq=1,
            command=MavCmd.MAV_CMD_NAV_WAYPOINT,
            target_system=drone_system_id,
            target_component=drone_component_id,
            frame=MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
            current=0,
            autocontinue=1,
            mission_type=mission_type,
        ),
    ]

    seq = 0

    # 1. GCS announces mission size.
    count = MissionCount(
        count=len(mission_items),
        target_system=drone_system_id,
        target_component=drone_component_id,
        mission_type=mission_type,
    )
    count_frame = frame_from_gcs(count, sequence=1)
    log_frame("GCS ->", count_frame)
    round_trip_message(dialect, count)

    # 2. Drone requests each mission item, GCS responds.
    while seq < len(mission_items):
        request = MissionRequest(
            seq=seq,
            target_system=gcs_system_id,
            target_component=gcs_component_id,
            mission_type=mission_type,
        )
        request_frame = frame_from_drone(request, sequence=seq + 10)
        log_frame("Drone ->", request_frame)
        round_trip_message(dialect, request)

        item = mission_items[seq]
        item_frame = frame_from_gcs(item, sequence=seq + 20)
        log_frame("GCS ->", item_frame)
        parsed_item = round_trip_message(dialect, item)
        if isinstance(parsed_item, MissionItem):
            print(f"  uploaded seq={parsed_item.seq} cmd={parsed_item.command}")

        seq += 1

    # 3. Drone accepts the mission.
    ack = MissionAck(
        target_system=gcs_system_id,
        target_component=gcs_component_id,
        type=MavMissionResult.MAV_MISSION_ACCEPTED,
        mission_type=mission_type,
    )
    ack_frame = frame_from_drone(ack, sequence=99)
    log_frame("Drone ->", ack_frame)
    parsed_ack = round_trip_message(dialect, ack)
    if isinstance(parsed_ack, MissionAck):
        print(f"Mission upload complete: {parsed_ack.type}")


if __name__ == "__main__":
    main()
