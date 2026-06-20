"""Shared helpers for MAVLink Python protocol examples."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

_ROOT = Path(__file__).resolve().parent.parent
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from mavlink_protocols import *  # noqa: E402, F403

# Ground control station identity (MAVLink convention).
gcs_system_id = 255
gcs_component_id = 190

# Simulated autopilot identity.
drone_system_id = 1
drone_component_id = 1


@dataclass(slots=True)
class VirtualLink:
    bus: VirtualMavlinkBus
    gcs: MavlinkSession
    drone: MavlinkSession
    dialect: MavlinkDialect


def create_virtual_link(dialect: MavlinkDialect) -> VirtualLink:
    bus = VirtualMavlinkBus()
    gcs_link = bus.create_endpoint()
    drone_link = bus.create_endpoint()

    gcs = MavlinkSession(
        dialect=dialect,
        link=gcs_link,
        system_id=gcs_system_id,
        component_id=gcs_component_id,
    )
    drone = MavlinkSession(
        dialect=dialect,
        link=drone_link,
        system_id=drone_system_id,
        component_id=drone_component_id,
    )

    return VirtualLink(bus=bus, gcs=gcs, drone=drone, dialect=dialect)


async def close_virtual_link(
    bus: VirtualMavlinkBus,
    gcs: MavlinkSession,
    drone: MavlinkSession,
) -> None:
    await gcs.close()
    await drone.close()
    await bus.close_all()
