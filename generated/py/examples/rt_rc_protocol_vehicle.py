#!/usr/bin/env python3
"""MavlinkGcs / MavlinkVehicleClient facade example for `rt_rc`."""

import asyncio
from datetime import timedelta

from mavlink import Heartbeat, MavParamType
from protocols_common import *


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    bus = VirtualMavlinkBus()
    gcs_link = bus.create_endpoint()
    drone_link = bus.create_endpoint()

    gcs = MavlinkGcs.connect(
        dialect=dialect,
        link=gcs_link,
        system_id=gcs_system_id,
        component_id=gcs_component_id,
    )

    drone_session = MavlinkSession(
        dialect=dialect,
        link=drone_link,
        system_id=drone_system_id,
        component_id=drone_component_id,
    )

    drone_publisher = HeartbeatPublisher(
        session=drone_session,
        heartbeat=HeartbeatTemplates.autopilot(mavlink_version=dialect.version),
        interval=timedelta(milliseconds=500),
    )

    parameter_server = ParameterServer(
        session=drone_session,
        initial_values={"SYSID_THISMAV": (1, MavParamType.MAV_PARAM_TYPE_INT32)},
    )

    command_server = CommandServer(session=drone_session)

    gcs.start()
    drone_publisher.start()

    client = await gcs.wait_for_vehicle(exclude_system_ids={gcs_system_id})
    print(f"Connected to vehicle {client.vehicle}")

    params = await client.parameters.fetch_all()
    print(f"Vehicle has {len(params)} parameters")

    ack = await client.command.request_message(Heartbeat.MSG_ID)
    print(f"REQUEST_MESSAGE ack: {ack.result}")

    await parameter_server.close()
    await command_server.close()
    drone_publisher.stop()
    await drone_session.close()
    await gcs.close()
    await bus.close_all()


if __name__ == "__main__":
    asyncio.run(main())
