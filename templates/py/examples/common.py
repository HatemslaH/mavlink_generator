"""Shared helpers for MAVLink Python examples."""

from __future__ import annotations

import sys
from pathlib import Path

_ROOT = Path(__file__).resolve().parent.parent
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from mavlink import *  # noqa: E402, F403

# Ground control station identity (MAVLink convention).
gcs_system_id = 255
gcs_component_id = 190

# Simulated autopilot identity.
drone_system_id = 1
drone_component_id = 1


def frame_from_gcs(message: MavlinkMessage, sequence: int = 0) -> MavlinkFrame:
    return MavlinkFrame.v2(sequence, gcs_system_id, gcs_component_id, message)


def frame_from_drone(message: MavlinkMessage, sequence: int = 0) -> MavlinkFrame:
    return MavlinkFrame.v2(sequence, drone_system_id, drone_component_id, message)


def param_id_from_string(name: str) -> list[int]:
    param_id: list[int] = []
    for code_unit in name.encode("ascii", errors="ignore")[:16]:
        param_id.append(code_unit)
    while len(param_id) < 16:
        param_id.append(0)
    return param_id


def param_id_to_string(param_id: list[int]) -> str:
    end = next((i for i, value in enumerate(param_id) if value == 0), len(param_id))
    return bytes(param_id[:end]).decode("ascii", errors="ignore")


def log_frame(direction: str, frame: MavlinkFrame) -> None:
    print(
        f"{direction} msgId={frame.message.mavlink_message_id} "
        f"sys={frame.system_id} comp={frame.component_id}"
    )


def round_trip_message(
    dialect: MavlinkDialect, message: MavlinkMessage
) -> MavlinkMessage | None:
    return dialect.parse(message.mavlink_message_id, message.serialize())
