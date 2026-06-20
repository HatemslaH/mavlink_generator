#!/usr/bin/env python3
"""Parameter protocol example for the `rt_rc` dialect."""

import asyncio

from mavlink import MavParamType
from protocols_common import *


async def main() -> None:
    dialect = MavlinkDialectRt_rc()
    link = create_virtual_link(dialect)

    parameter_server = ParameterServer(
        session=link.drone,
        initial_values={
            "SYSID_THISMAV": (1, MavParamType.MAV_PARAM_TYPE_INT32),
            "SYSID_MYGCS": (255, MavParamType.MAV_PARAM_TYPE_INT32),
            "COMPASS_ENABLE": (1, MavParamType.MAV_PARAM_TYPE_INT32),
        },
    )

    parameter_protocol = ParameterProtocol(
        session=link.gcs,
        target_system=drone_system_id,
        target_component=drone_component_id,
    )

    all_params = await parameter_protocol.fetch_all(
        on_progress=lambda entry, received, expected: print(
            f"  [{received}/{expected}] {entry.id}={entry.value}"
        ),
    )
    print(
        f"Fetched {len(all_params)} parameters "
        f"(cache size={len(parameter_protocol.cache)})"
    )

    single = await parameter_protocol.read_by_name("SYSID_THISMAV")
    print(f"Read SYSID_THISMAV={single.value}")

    updated = await parameter_protocol.write_by_name("COMPASS_ENABLE", 0)
    print(f"Wrote COMPASS_ENABLE={updated.value} ({updated.type})")

    await parameter_server.close()
    await close_virtual_link(bus=link.bus, gcs=link.gcs, drone=link.drone)


if __name__ == "__main__":
    asyncio.run(main())
