from __future__ import annotations

import bindings  # noqa: F401

from mavlink_protocols import (  # noqa: E402
    CommandProtocol,
    HeartbeatMonitor,
    HeartbeatPublisher,
    MavlinkCancellationToken,
    MavlinkGcs,
    MavlinkNode,
    MavlinkSession,
    MavlinkVehicleClient,
    MissionProtocol,
    ParameterProtocol,
)

gcs_system_id = 255
gcs_component_id = 190


class GcsContext:
    """Shared MAVLink GCS state for the interactive SITL example."""

    def __init__(
        self,
        gcs: MavlinkGcs,
        vehicle: MavlinkNode,
        client: MavlinkVehicleClient,
    ) -> None:
        self.gcs = gcs
        self.vehicle = vehicle
        self.client = client
        self.operation_cancel: MavlinkCancellationToken | None = None

    @property
    def session(self) -> MavlinkSession:
        return self.gcs.session

    @property
    def heartbeat_monitor(self) -> HeartbeatMonitor:
        return self.gcs.heartbeat_monitor

    @property
    def heartbeat_publisher(self) -> HeartbeatPublisher:
        return self.gcs.heartbeat_publisher

    @property
    def parameters(self) -> ParameterProtocol:
        return self.client.parameters

    @property
    def mission(self) -> MissionProtocol:
        return self.client.mission

    @property
    def command(self) -> CommandProtocol:
        return self.client.command

    @property
    def target_system(self) -> int:
        return self.vehicle.system_id

    @property
    def target_component(self) -> int:
        return self.vehicle.component_id
