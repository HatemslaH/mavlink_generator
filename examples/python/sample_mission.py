from __future__ import annotations

import bindings  # noqa: F401

from mavlink import MavCmd, MissionItemInt  # noqa: E402
from mavlink_protocols import MissionItems  # noqa: E402


def build_sample_mission(
    *,
    target_system: int,
    target_component: int,
) -> list[MissionItemInt]:
    """Hardcoded sample mission (Zurich area coordinates, same as virtual examples)."""
    return MissionItems.with_sequential_seq(
        [
            MissionItems.waypoint(
                seq=0,
                latitude=47.397742,
                longitude=8.545594,
                altitude=50,
                target_system=target_system,
                target_component=target_component,
            ),
            MissionItems.waypoint(
                seq=1,
                latitude=47.398000,
                longitude=8.546000,
                altitude=50,
                target_system=target_system,
                target_component=target_component,
            ),
            MissionItems.waypoint(
                seq=2,
                latitude=47.398258,
                longitude=8.546406,
                altitude=50,
                target_system=target_system,
                target_component=target_component,
                command=MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
            ),
        ]
    )


def describe_mission_item(item: MissionItemInt) -> str:
    lat = item.x / 1e7
    lon = item.y / 1e7
    return (
        f"seq={item.seq} {item.command.name} "
        f"lat={lat:.6f} lon={lon:.6f} alt={item.z}m"
    )
