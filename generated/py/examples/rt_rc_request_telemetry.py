#!/usr/bin/env python3
"""Virtual telemetry request for the `rt_rc` dialect.

Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
"""

from common import *


def main() -> None:
    dialect = MavlinkDialectRt_rc()

    # Stream ATTITUDE (msg id 30) at 10 Hz (100_000 microseconds).
    set_interval = CommandLong(
        param1=float(Attitude.MSG_ID),
        param2=100000,
        param3=0,
        param4=0,
        param5=0,
        param6=0,
        param7=0,
        command=MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
        target_system=drone_system_id,
        target_component=drone_component_id,
        confirmation=0,
    )
    interval_frame = frame_from_gcs(set_interval, sequence=1)
    log_frame("GCS ->", interval_frame)
    parsed_interval = round_trip_message(dialect, set_interval)
    if isinstance(parsed_interval, CommandLong):
        print(
            f"  SET_MESSAGE_INTERVAL msgId={int(parsed_interval.param1)} "
            f"interval_us={int(parsed_interval.param2)}"
        )

    # One-shot ATTITUDE sample via MAV_CMD_REQUEST_MESSAGE.
    request_once = CommandLong(
        param1=float(Attitude.MSG_ID),
        param2=0,
        param3=0,
        param4=0,
        param5=0,
        param6=0,
        param7=0,
        command=MavCmd.MAV_CMD_REQUEST_MESSAGE,
        target_system=drone_system_id,
        target_component=drone_component_id,
        confirmation=0,
    )
    once_frame = frame_from_gcs(request_once, sequence=2)
    log_frame("GCS ->", once_frame)
    round_trip_message(dialect, request_once)

    # Simulated vehicle response: ATTITUDE telemetry frame.
    attitude = Attitude(
        time_boot_ms=12345,
        roll=0.01,
        pitch=-0.02,
        yaw=1.57,
        rollspeed=0,
        pitchspeed=0,
        yawspeed=0,
    )
    telemetry_frame = frame_from_drone(attitude, sequence=3)
    log_frame("Drone ->", telemetry_frame)
    parsed_attitude = round_trip_message(dialect, attitude)
    if isinstance(parsed_attitude, Attitude):
        print(
            f"  ATTITUDE roll={parsed_attitude.roll} "
            f"pitch={parsed_attitude.pitch} yaw={parsed_attitude.yaw}"
        )


if __name__ == "__main__":
    main()
