#!/usr/bin/env python3
"""Virtual parameter service for the `rt_rc` dialect.

Follows https://mavlink.io/en/services/parameter.html:
PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
"""

from dataclasses import dataclass

from common import *


@dataclass
class SimulatedParam:
    id: str
    value: float
    index: int


def main() -> None:
    dialect = MavlinkDialectRt_rc()

    # 1. GCS requests the full onboard parameter set.
    list_request = ParamRequestList(
        target_system=drone_system_id,
        target_component=drone_component_id,
    )
    list_frame = frame_from_gcs(list_request, sequence=1)
    log_frame("GCS ->", list_frame)
    round_trip_message(dialect, list_request)

    # 2. Drone responds with PARAM_VALUE messages (simulated subset).
    simulated_params = [
        SimulatedParam("SYSID_THISMAV", 1, 0),
        SimulatedParam("SYSID_MYGCS", 255, 1),
        SimulatedParam("COMPASS_ENABLE", 1, 2),
    ]

    for param in simulated_params:
        value = ParamValue(
            param_value=param.value,
            param_count=len(simulated_params),
            param_index=param.index,
            param_id=param_id_from_string(param.id),
            param_type=MavParamType.MAV_PARAM_TYPE_REAL32,
        )
        value_frame = frame_from_drone(value, sequence=param.index + 10)
        log_frame("Drone ->", value_frame)
        parsed = round_trip_message(dialect, value)
        if isinstance(parsed, ParamValue):
            print(
                f"  PARAM_VALUE [{param.index + 1}/{len(simulated_params)}] "
                f"{param_id_to_string(parsed.param_id)}={parsed.param_value}"
            )

    # 3. GCS requests one parameter by name (param_index = -1).
    param_name = "SYSID_THISMAV"
    read_request = ParamRequestRead(
        param_index=-1,
        target_system=drone_system_id,
        target_component=drone_component_id,
        param_id=param_id_from_string(param_name),
    )
    read_frame = frame_from_gcs(read_request, sequence=50)
    log_frame("GCS ->", read_frame)
    parsed_read = round_trip_message(dialect, read_request)
    if isinstance(parsed_read, ParamRequestRead):
        print(f"  PARAM_REQUEST_READ id={param_id_to_string(parsed_read.param_id)}")

    # 4. Drone answers with the matching PARAM_VALUE.
    single_value = ParamValue(
        param_value=1,
        param_count=len(simulated_params),
        param_index=0,
        param_id=param_id_from_string(param_name),
        param_type=MavParamType.MAV_PARAM_TYPE_REAL32,
    )
    single_frame = frame_from_drone(single_value, sequence=51)
    log_frame("Drone ->", single_frame)
    round_trip_message(dialect, single_value)


if __name__ == "__main__":
    main()
