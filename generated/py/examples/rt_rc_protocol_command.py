#!/usr/bin/env python3
"""Command protocol example for the `rt_rc` dialect."""

import asyncio

from mavlink import Attitude, CommandLong, MavResult
from protocols_common import *


async def on_command_long(command: CommandLong) -> MavResult:
    print(
        f"Vehicle received COMMAND_LONG: {command.command} "
        f"p1={command.param1} p2={command.param2}"
    )
    return MavResult.MAV_RESULT_ACCEPTED


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    link = create_virtual_link(dialect)

    command_server = CommandServer(session=link.drone, on_command_long=on_command_long)

    command_protocol = CommandProtocol(
        session=link.gcs,
        target_system=drone_system_id,
        target_component=drone_component_id,
    )

    interval_ack = await command_protocol.set_message_interval(Attitude.MSG_ID, 100000)
    print(f"SET_MESSAGE_INTERVAL ack: {interval_ack.result}")

    request_ack = await command_protocol.request_message(Attitude.MSG_ID)
    print(f"REQUEST_MESSAGE ack: {request_ack.result}")

    arm_ack = await command_protocol.arm()
    print(f"ARM ack: {arm_ack.result}")

    disarm_ack = await command_protocol.disarm()
    print(f"DISARM ack: {disarm_ack.result}")

    await command_server.close()
    await close_virtual_link(bus=link.bus, gcs=link.gcs, drone=link.drone)


if __name__ == "__main__":
    asyncio.run(main())
