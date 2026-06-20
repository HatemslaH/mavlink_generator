#!/usr/bin/env python3
"""Heartbeat protocol example for the `rt_rc` dialect."""

import asyncio
from datetime import timedelta

from protocols_common import *


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    link = create_virtual_link(dialect)

    gcs_publisher = HeartbeatPublisher(
        session=link.gcs,
        heartbeat=HeartbeatTemplates.gcs(mavlink_version=dialect.version),
        interval=timedelta(milliseconds=500),
    )

    drone_publisher = HeartbeatPublisher(
        session=link.drone,
        heartbeat=HeartbeatTemplates.autopilot(mavlink_version=dialect.version),
        interval=timedelta(milliseconds=500),
    )

    gcs_monitor = HeartbeatMonitor(
        session=link.gcs,
        timeout=timedelta(seconds=2),
    )

    gcs_monitor.start()
    gcs_publisher.start()
    drone_publisher.start()

    vehicle = await gcs_monitor.wait_for_vehicle(
        exclude_system_ids={gcs_system_id},
        timeout=timedelta(seconds=5),
    )
    print(f"Vehicle discovered: {vehicle}")
    print(f"Drone online: {gcs_monitor.is_online(vehicle)}")
    state = gcs_monitor.state_for(vehicle)
    if state is not None:
        print(
            f"Drone heartbeat: type={state.heartbeat.type} "
            f"status={state.heartbeat.system_status}"
        )

    drone_publisher.stop()
    await asyncio.sleep(2.5)
    print(f"Drone online after stop: {gcs_monitor.is_online(vehicle)}")

    await gcs_monitor.stop()
    gcs_publisher.stop()

    await close_virtual_link(bus=link.bus, gcs=link.gcs, drone=link.drone)


if __name__ == "__main__":
    asyncio.run(main())
