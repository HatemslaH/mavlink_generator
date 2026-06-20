from __future__ import annotations

from datetime import timedelta

from mavlink_dialect import MavlinkDialect

from .command_protocol import CommandProtocol
from .heartbeat_protocol import (
    HeartbeatMonitor,
    HeartbeatPublisher,
    HeartbeatTemplates,
    MavlinkNode,
)
from .mavlink_link import MavlinkLink
from .mavlink_session import MavlinkSession
from .mission_protocol import MissionProtocol
from .parameter_protocol import ParameterProtocol


class MavlinkVehicleClient:
    def __init__(
        self,
        session: MavlinkSession,
        vehicle: MavlinkNode,
        parameter_request_timeout: timedelta = timedelta(seconds=10),
        parameter_idle_timeout: timedelta = timedelta(seconds=2),
        mission_item_timeout: timedelta = timedelta(seconds=10),
        mission_operation_timeout: timedelta = timedelta(seconds=30),
        command_timeout: timedelta = timedelta(seconds=10),
    ) -> None:
        self.session = session
        self.vehicle = vehicle
        self.parameters = ParameterProtocol(
            session=session,
            target_system=vehicle.system_id,
            target_component=vehicle.component_id,
            request_timeout=parameter_request_timeout,
            idle_timeout=parameter_idle_timeout,
        )
        self.mission = MissionProtocol(
            session=session,
            target_system=vehicle.system_id,
            target_component=vehicle.component_id,
            item_timeout=mission_item_timeout,
            operation_timeout=mission_operation_timeout,
        )
        self.command = CommandProtocol(
            session=session,
            target_system=vehicle.system_id,
            target_component=vehicle.component_id,
            default_timeout=command_timeout,
        )

    @property
    def target_system(self) -> int:
        return self.vehicle.system_id

    @property
    def target_component(self) -> int:
        return self.vehicle.component_id


class MavlinkGcs:
    def __init__(
        self,
        session: MavlinkSession,
        heartbeat_publisher: HeartbeatPublisher,
        heartbeat_monitor: HeartbeatMonitor,
    ) -> None:
        self.session = session
        self.heartbeat_publisher = heartbeat_publisher
        self.heartbeat_monitor = heartbeat_monitor

    def start(self) -> None:
        self.heartbeat_monitor.start()
        self.heartbeat_publisher.start()

    async def stop_heartbeats(self) -> None:
        self.heartbeat_publisher.stop()
        await self.heartbeat_monitor.stop()

    async def wait_for_vehicle(
        self,
        exclude_system_ids: set[int] | None = None,
        timeout: timedelta = timedelta(seconds=60),
    ) -> MavlinkVehicleClient:
        node = await self.heartbeat_monitor.wait_for_vehicle(
            exclude_system_ids=exclude_system_ids,
            timeout=timeout,
        )
        return self.vehicle_client(node)

    def vehicle_client(self, vehicle: MavlinkNode) -> MavlinkVehicleClient:
        return MavlinkVehicleClient(session=self.session, vehicle=vehicle)

    @classmethod
    def connect(
        cls,
        dialect: MavlinkDialect,
        link: MavlinkLink,
        system_id: int = 255,
        component_id: int = 190,
        heartbeat_interval: timedelta = timedelta(seconds=1),
        heartbeat_timeout: timedelta = timedelta(seconds=3),
    ) -> MavlinkGcs:
        session = MavlinkSession(
            dialect=dialect,
            link=link,
            system_id=system_id,
            component_id=component_id,
        )
        return cls(
            session=session,
            heartbeat_publisher=HeartbeatPublisher(
                session=session,
                heartbeat=HeartbeatTemplates.gcs(mavlink_version=dialect.version),
                interval=heartbeat_interval,
            ),
            heartbeat_monitor=HeartbeatMonitor(
                session=session,
                timeout=heartbeat_timeout,
            ),
        )

    async def close(self) -> None:
        await self.stop_heartbeats()
        await self.session.close()
