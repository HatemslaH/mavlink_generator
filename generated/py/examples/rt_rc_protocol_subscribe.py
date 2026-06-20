#!/usr/bin/env python3
"""Typed message subscription example for the `rt_rc` dialect."""

import asyncio

from mavlink import Attitude
from protocols_common import *


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    link = create_virtual_link(dialect)
    vehicle = MavlinkNode(drone_system_id, drone_component_id)

    attitude_samples: list[Attitude] = []
    subscription = link.gcs.listen_message(
        Attitude,
        lambda message, frame: attitude_samples.append(message),
        from_system_id=vehicle.system_id,
    )

    await link.drone.send(
        Attitude(
            time_boot_ms=1000,
            roll=0.1,
            pitch=-0.05,
            yaw=1.57,
            rollspeed=0,
            pitchspeed=0,
            yawspeed=0,
        )
    )

    await asyncio.sleep(0.05)
    subscription.cancel()

    print(f"Received {len(attitude_samples)} ATTITUDE samples via listen_message")
    if attitude_samples:
        sample = attitude_samples[0]
        print(f"  roll={sample.roll} pitch={sample.pitch} yaw={sample.yaw}")

    await close_virtual_link(bus=link.bus, gcs=link.gcs, drone=link.drone)


if __name__ == "__main__":
    asyncio.run(main())
