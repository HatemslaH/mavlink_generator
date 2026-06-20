#!/usr/bin/env python3
"""Mission protocol example for the `rt_rc` dialect.

Uses MissionProtocol on the GCS side and MissionServer on the vehicle side
over a transport-agnostic in-memory VirtualMavlinkBus.
"""

import asyncio

from protocols_common import *


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    link = create_virtual_link(dialect)

    mission_server = MissionServer(session=link.drone)
    command_server = CommandServer(session=link.drone)
    mission_protocol = MissionProtocol(
        session=link.gcs,
        target_system=drone_system_id,
        target_component=drone_component_id,
    )

    plan = [
        MissionItems.waypoint(
            seq=0,
            latitude=47.397742,
            longitude=8.545594,
            altitude=50,
            target_system=drone_system_id,
            target_component=drone_component_id,
        ),
        MissionItems.waypoint(
            seq=1,
            latitude=47.398000,
            longitude=8.546000,
            altitude=50,
            target_system=drone_system_id,
            target_component=drone_component_id,
        ),
    ]

    upload_result = await mission_protocol.upload(
        plan,
        on_progress=lambda sent, total, item: print(
            f"Upload progress {sent}/{total} seq={item.seq} cmd={item.command}"
        ),
    )
    print(f"Mission upload result: {upload_result}")
    print(f"Vehicle stored {len(mission_server.items)} items")

    downloaded = await mission_protocol.download(
        on_progress=lambda received, total, item: print(
            f"Download progress {received}/{total} seq={item.seq}"
        ),
    )
    print(f"Downloaded {len(downloaded)} mission items")

    command_protocol = CommandProtocol(
        session=link.gcs,
        target_system=drone_system_id,
        target_component=drone_component_id,
    )
    set_current = await mission_protocol.set_current_with_command(
        0,
        command=command_protocol,
    )
    ack_result = set_current.command_ack.result if set_current.command_ack else None
    print(f"Set current seq={set_current.sequence} ack={ack_result}")

    clear_result = await mission_protocol.clear()
    print(f"Mission clear result: {clear_result}")

    await mission_server.close()
    await command_server.close()
    await close_virtual_link(bus=link.bus, gcs=link.gcs, drone=link.drone)


if __name__ == "__main__":
    asyncio.run(main())
