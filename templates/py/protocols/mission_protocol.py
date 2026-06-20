from __future__ import annotations

import asyncio
from collections.abc import Callable
from dataclasses import dataclass
from datetime import timedelta

from mavlink import (
    CommandAck,
    CommandInt,
    CommandLong,
    MavCmd,
    MavComponent,
    MavMissionResult,
    MavResult,
    MissionAck,
    MissionClearAll,
    MissionCount,
    MissionItem,
    MissionItemInt,
    MissionRequest,
    MissionRequestInt,
    MissionRequestList,
    MissionSetCurrent,
)
from mavlink_frame import MavlinkFrame
from mavlink_message import MavlinkMessage

from .command_protocol import CommandProtocol
from .mavlink_cancellation import MavlinkCancellationToken
from .mavlink_session import MavlinkSession


class MissionItems:
    @staticmethod
    def waypoint(
        seq: int,
        latitude: float,
        longitude: float,
        altitude: float,
        target_system: int,
        target_component: int,
        command: MavCmd = MavCmd.MAV_CMD_NAV_WAYPOINT,
        frame=None,
        mission_type=None,
        param1: float = 0,
        param2: float = 0,
        param3: float = 0,
        param4: float = 0,
        current: int = 0,
        autocontinue: int = 1,
    ) -> MissionItemInt:
        from mavlink import MavFrame, MavMissionType  # noqa: PLC0415

        resolved_frame = frame or MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT_INT
        resolved_mission_type = mission_type or MavMissionType.MAV_MISSION_TYPE_MISSION
        return MissionItemInt(
            param1=param1,
            param2=param2,
            param3=param3,
            param4=param4,
            x=int(latitude * 1e7),
            y=int(longitude * 1e7),
            z=altitude,
            seq=seq,
            command=command,
            target_system=target_system,
            target_component=target_component,
            frame=resolved_frame,
            current=current,
            autocontinue=autocontinue,
            mission_type=resolved_mission_type,
        )

    @staticmethod
    def to_legacy_item(item: MissionItemInt) -> MissionItem:
        return MissionItem(
            param1=item.param1,
            param2=item.param2,
            param3=item.param3,
            param4=item.param4,
            x=item.x / 1e7,
            y=item.y / 1e7,
            z=item.z,
            seq=item.seq,
            command=item.command,
            target_system=item.target_system,
            target_component=item.target_component,
            frame=item.frame,
            current=item.current,
            autocontinue=item.autocontinue,
            mission_type=item.mission_type,
        )

    @staticmethod
    def from_legacy_item(item: MissionItem) -> MissionItemInt:
        return MissionItemInt(
            param1=item.param1,
            param2=item.param2,
            param3=item.param3,
            param4=item.param4,
            x=int(item.x * 1e7),
            y=int(item.y * 1e7),
            z=item.z,
            seq=item.seq,
            command=item.command,
            target_system=item.target_system,
            target_component=item.target_component,
            frame=item.frame,
            current=item.current,
            autocontinue=item.autocontinue,
            mission_type=item.mission_type,
        )

    @staticmethod
    def with_sequential_seq(items: list[MissionItemInt]) -> list[MissionItemInt]:
        return [
            MissionItemInt(
                param1=item.param1,
                param2=item.param2,
                param3=item.param3,
                param4=item.param4,
                x=item.x,
                y=item.y,
                z=item.z,
                seq=index,
                command=item.command,
                target_system=item.target_system,
                target_component=item.target_component,
                frame=item.frame,
                current=item.current,
                autocontinue=item.autocontinue,
                mission_type=item.mission_type,
            )
            for index, item in enumerate(items)
        ]


MissionUploadProgressCallback = Callable[[int, int, MissionItemInt], None]
MissionDownloadProgressCallback = Callable[[int, int, MissionItemInt], None]


@dataclass(frozen=True, slots=True)
class MissionSetCurrentResult:
    sequence: int
    command_ack: CommandAck | None = None


class MissionProtocol:
    def __init__(
        self,
        session: MavlinkSession,
        target_system: int,
        target_component: int,
        item_timeout: timedelta = timedelta(seconds=3),
        operation_timeout: timedelta = timedelta(seconds=10),
    ) -> None:
        self.session = session
        self.target_system = target_system
        self.target_component = target_component
        self.item_timeout = item_timeout
        self.operation_timeout = operation_timeout

    async def upload(
        self,
        items: list[MissionItemInt],
        mission_type=None,
        on_progress: MissionUploadProgressCallback | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ):
        from mavlink import MavMissionType  # noqa: PLC0415

        if cancel is not None:
            cancel.throw_if_cancelled()

        resolved_mission_type = mission_type or MavMissionType.MAV_MISSION_TYPE_MISSION
        plan = MissionItems.with_sequential_seq(items)

        await self.session.send(
            MissionCount(
                count=len(plan),
                target_system=self.target_system,
                target_component=self.target_component,
                mission_type=resolved_mission_type,
            )
        )

        for item in plan:
            if cancel is not None:
                cancel.throw_if_cancelled()

            request = await self.session.wait_for_message(
                predicate=lambda message: self._is_item_request(
                    message, item.seq, resolved_mission_type
                ),
                from_system_id=self.target_system,
                timeout=self.item_timeout,
                cancel=cancel,
            )

            if isinstance(request, MissionRequestInt):
                await self.session.send(item)
            elif isinstance(request, MissionRequest):
                await self.session.send(MissionItems.to_legacy_item(item))

            if on_progress is not None:
                on_progress(item.seq + 1, len(plan), item)

        ack = await self.session.wait_for_message_type(
            MissionAck,
            from_system_id=self.target_system,
            timeout=self.operation_timeout,
            cancel=cancel,
        )
        return ack.type

    async def download(
        self,
        mission_type=None,
        on_progress: MissionDownloadProgressCallback | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> list[MissionItemInt]:
        from mavlink import MavMissionType  # noqa: PLC0415

        if cancel is not None:
            cancel.throw_if_cancelled()

        resolved_mission_type = mission_type or MavMissionType.MAV_MISSION_TYPE_MISSION

        await self.session.send(
            MissionRequestList(
                target_system=self.target_system,
                target_component=self.target_component,
                mission_type=resolved_mission_type,
            )
        )

        count_message = await self.session.wait_for_message_type(
            MissionCount,
            from_system_id=self.target_system,
            timeout=self.operation_timeout,
            cancel=cancel,
        )

        items: list[MissionItemInt] = []

        for seq in range(count_message.count):
            if cancel is not None:
                cancel.throw_if_cancelled()

            await self.session.send(
                MissionRequestInt(
                    seq=seq,
                    target_system=self.target_system,
                    target_component=self.target_component,
                    mission_type=resolved_mission_type,
                )
            )

            item_message = await self.session.wait_for_message(
                predicate=lambda message: (
                    (
                        isinstance(message, MissionItemInt)
                        and message.seq == seq
                        and message.mission_type == resolved_mission_type
                    )
                    or (
                        isinstance(message, MissionItem)
                        and message.seq == seq
                        and message.mission_type == resolved_mission_type
                    )
                ),
                from_system_id=self.target_system,
                timeout=self.item_timeout,
                cancel=cancel,
            )

            if isinstance(item_message, MissionItemInt):
                item = item_message
            else:
                item = MissionItems.from_legacy_item(item_message)  # type: ignore[arg-type]

            items.append(item)
            if on_progress is not None:
                on_progress(len(items), count_message.count, item)

        await self.session.send(
            MissionAck(
                target_system=self.target_system,
                target_component=self.target_component,
                type=MavMissionResult.MAV_MISSION_ACCEPTED,
                mission_type=resolved_mission_type,
            )
        )

        return items

    async def clear(
        self,
        mission_type=None,
        cancel: MavlinkCancellationToken | None = None,
    ):
        from mavlink import MavMissionType  # noqa: PLC0415

        resolved_mission_type = mission_type or MavMissionType.MAV_MISSION_TYPE_MISSION

        await self.session.send(
            MissionClearAll(
                target_system=self.target_system,
                target_component=self.target_component,
                mission_type=resolved_mission_type,
            )
        )

        ack = await self.session.wait_for_message_type(
            MissionAck,
            from_system_id=self.target_system,
            timeout=self.operation_timeout,
            cancel=cancel,
        )
        return ack.type

    async def set_current(
        self, seq: int, cancel: MavlinkCancellationToken | None = None
    ) -> None:
        if cancel is not None:
            cancel.throw_if_cancelled()
        await self.session.send(
            MissionSetCurrent(
                seq=seq,
                target_system=self.target_system,
                target_component=self.target_component,
            )
        )

    async def set_current_with_command(
        self,
        seq: int,
        command: CommandProtocol | None = None,
        also_send_command: bool = True,
        reset_mission: bool = False,
        cancel: MavlinkCancellationToken | None = None,
    ) -> MissionSetCurrentResult:
        if cancel is not None:
            cancel.throw_if_cancelled()
        await self.set_current(seq, cancel=cancel)

        ack = None
        if also_send_command and command is not None:
            ack = await command.set_mission_current(
                seq, reset_mission=reset_mission, cancel=cancel
            )

        return MissionSetCurrentResult(sequence=seq, command_ack=ack)

    def _is_item_request(
        self, message: MavlinkMessage, seq: int, mission_type
    ) -> bool:
        if isinstance(message, MissionRequestInt):
            return message.seq == seq and message.mission_type == mission_type
        if isinstance(message, MissionRequest):
            return message.seq == seq and message.mission_type == mission_type
        return False


class MissionServer:
    def __init__(
        self,
        session: MavlinkSession,
        initial_mission: list[MissionItemInt] | None = None,
        mission_type=None,
    ) -> None:
        from mavlink import MavMissionType  # noqa: PLC0415

        self.session = session
        self.mission_type = mission_type or MavMissionType.MAV_MISSION_TYPE_MISSION
        self._items: list[MissionItemInt] = list(initial_mission or [])
        self._incoming: dict[int, MissionItemInt] = {}
        self._incoming_count: int | None = None
        self._frame_task = asyncio.create_task(self._listen_frames())

    @property
    def items(self) -> list[MissionItemInt]:
        return list(self._items)

    async def close(self) -> None:
        self._frame_task.cancel()
        try:
            await self._frame_task
        except asyncio.CancelledError:
            pass

    def replace_mission(self, items: list[MissionItemInt]) -> None:
        self._items = MissionItems.with_sequential_seq(items)
        self._incoming.clear()
        self._incoming_count = None

    async def _listen_frames(self) -> None:
        async for frame in self.session.frames():
            await self._on_frame(frame)

    async def _on_frame(self, frame: MavlinkFrame) -> None:
        message = frame.message

        if isinstance(message, MissionCount) and self._targets_us(message):
            if message.mission_type != self.mission_type:
                return
            self._incoming_count = message.count
            self._incoming.clear()
            if message.count > 0:
                await self._request_upload_item(frame, 0)
            else:
                await self._send_upload_ack(frame)
            return

        if isinstance(message, MissionItemInt) and self._targets_us(message):
            if message.mission_type != self.mission_type:
                return
            await self._store_incoming_item(frame, message)
            return

        if isinstance(message, MissionItem) and self._targets_us(message):
            if message.mission_type != self.mission_type:
                return
            await self._store_incoming_item(frame, MissionItems.from_legacy_item(message))
            return

        if isinstance(message, MissionRequestInt) and self._targets_us(message):
            await self._send_requested_item(frame, message.seq)
            return

        if isinstance(message, MissionRequest) and self._targets_us(message):
            await self._send_requested_item(frame, message.seq)
            return

        if isinstance(message, MissionRequestList) and self._targets_us(message):
            if message.mission_type != self.mission_type:
                return
            await self.session.send(
                MissionCount(
                    count=len(self._items),
                    target_system=frame.system_id,
                    target_component=frame.component_id,
                    mission_type=self.mission_type,
                )
            )
            return

        if isinstance(message, MissionClearAll) and self._targets_us(message):
            if message.mission_type != self.mission_type:
                return
            self._items.clear()
            self._incoming.clear()
            self._incoming_count = None
            await self.session.send(
                MissionAck(
                    target_system=frame.system_id,
                    target_component=frame.component_id,
                    type=MavMissionResult.MAV_MISSION_ACCEPTED,
                    mission_type=self.mission_type,
                )
            )

    async def _store_incoming_item(self, frame: MavlinkFrame, item: MissionItemInt) -> None:
        self._incoming[item.seq] = item
        expected = self._incoming_count
        if expected is None:
            return

        if len(self._incoming) < expected:
            await self._request_upload_item(frame, item.seq + 1)
            return

        self._items = [self._incoming[index] for index in range(expected)]
        self._incoming.clear()
        self._incoming_count = None
        await self._send_upload_ack(frame)

    async def _request_upload_item(self, request_frame: MavlinkFrame, seq: int) -> None:
        await self.session.send(
            MissionRequestInt(
                seq=seq,
                target_system=request_frame.system_id,
                target_component=request_frame.component_id,
                mission_type=self.mission_type,
            )
        )

    async def _send_upload_ack(self, request_frame: MavlinkFrame) -> None:
        await self.session.send(
            MissionAck(
                target_system=request_frame.system_id,
                target_component=request_frame.component_id,
                type=MavMissionResult.MAV_MISSION_ACCEPTED,
                mission_type=self.mission_type,
            )
        )

    async def _send_requested_item(self, request_frame: MavlinkFrame, seq: int) -> None:
        if seq < 0 or seq >= len(self._items):
            await self.session.send(
                MissionAck(
                    target_system=request_frame.system_id,
                    target_component=request_frame.component_id,
                    type=MavMissionResult.MAV_MISSION_INVALID_SEQUENCE,
                    mission_type=self.mission_type,
                )
            )
            return

        await self.session.send(self._items[seq])

    def _targets_us(self, message: MavlinkMessage) -> bool:
        target_system = getattr(message, "target_system", None)
        target_component = getattr(message, "target_component", None)
        if target_system is None or target_component is None:
            return False
        return self._matches_target(target_system, target_component)

    def _matches_target(self, target_system: int, target_component: int) -> bool:
        if target_system != self.session.system_id and target_system != 0:
            return False
        if (
            target_component != self.session.component_id
            and target_component != MavComponent.MAV_COMP_ID_ALL
        ):
            return False
        return True
