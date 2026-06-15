#!/usr/bin/env python3
"""Example for the `rt_rc` dialect: serialize a Heartbeat frame and parse it back."""

from common import *


def main() -> None:
    dialect = MavlinkDialectRt_rc()

    heartbeat = Heartbeat(
        custom_mode=0,
        type=MavType.MAV_TYPE_QUADROTOR,
        autopilot=MavAutopilot.MAV_AUTOPILOT_PX4,
        base_mode=0,
        system_status=MavState.MAV_STATE_ACTIVE,
        mavlink_version=dialect.version,
    )

    frame = frame_from_gcs(heartbeat)
    wire = frame.serialize()
    log_frame("GCS ->", frame)
    print(f"Serialized HEARTBEAT ({len(wire)} bytes)")

    parsed = round_trip_message(dialect, heartbeat)
    if isinstance(parsed, Heartbeat):
        print(f"Parsed HEARTBEAT type={parsed.type} status={parsed.system_status}")


if __name__ == "__main__":
    main()
