from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable
from datetime import timedelta

from mavlink import CommandAck, CommandInt, CommandLong, MavCmd, MavResult
from mavlink_frame import MavlinkFrame

from .mavlink_cancellation import MavlinkCancellationToken
from .mavlink_session import MavlinkSession


class CommandProtocol:
    def __init__(
        self,
        session: MavlinkSession,
        target_system: int,
        target_component: int,
        default_timeout: timedelta = timedelta(seconds=5),
    ) -> None:
        self.session = session
        self.target_system = target_system
        self.target_component = target_component
        self.default_timeout = default_timeout

    async def send_long(
        self,
        command: CommandLong,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        await self.session.send(command)
        return await self.wait_for_ack(command.command, timeout=timeout, cancel=cancel)

    async def send_int(
        self,
        command: CommandInt,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        await self.session.send(command)
        return await self.wait_for_ack(command.command, timeout=timeout, cancel=cancel)

    async def command_long(
        self,
        command: MavCmd,
        param1: float = 0,
        param2: float = 0,
        param3: float = 0,
        param4: float = 0,
        param5: float = 0,
        param6: float = 0,
        param7: float = 0,
        confirmation: int = 0,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.send_long(
            CommandLong(
                param1=param1,
                param2=param2,
                param3=param3,
                param4=param4,
                param5=param5,
                param6=param6,
                param7=param7,
                command=command,
                target_system=self.target_system,
                target_component=self.target_component,
                confirmation=confirmation,
            ),
            timeout=timeout,
            cancel=cancel,
        )

    async def request_message(
        self,
        message_id: int,
        param2: float = 0,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_REQUEST_MESSAGE,
            param1=float(message_id),
            param2=param2,
            timeout=timeout,
            cancel=cancel,
        )

    async def set_message_interval(
        self,
        message_id: int,
        interval_us: int,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
            param1=float(message_id),
            param2=float(interval_us),
            timeout=timeout,
            cancel=cancel,
        )

    async def stop_message_interval(
        self,
        message_id: int,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.set_message_interval(
            message_id, 0, timeout=timeout, cancel=cancel
        )

    async def set_mission_current(
        self,
        sequence: int,
        reset_mission: bool = False,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_DO_SET_MISSION_CURRENT,
            param1=float(sequence),
            param2=1.0 if reset_mission else 0.0,
            timeout=timeout,
            cancel=cancel,
        )

    async def arm(
        self,
        force: bool = False,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
            param1=1,
            param2=21196 if force else 0,
            timeout=timeout,
            cancel=cancel,
        )

    async def disarm(
        self,
        force: bool = False,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
            param1=0,
            param2=21196 if force else 0,
            timeout=timeout,
            cancel=cancel,
        )

    async def takeoff(
        self,
        altitude: float = 10,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_NAV_TAKEOFF,
            param7=altitude,
            timeout=timeout,
            cancel=cancel,
        )

    async def land(
        self,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_NAV_LAND,
            timeout=timeout,
            cancel=cancel,
        )

    async def return_to_launch(
        self,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        return await self.command_long(
            command=MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
            timeout=timeout,
            cancel=cancel,
        )

    async def wait_for_ack(
        self,
        command: MavCmd,
        timeout: timedelta | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> CommandAck:
        message = await self.session.wait_for_message(
            predicate=lambda msg: isinstance(msg, CommandAck) and msg.command == command,
            from_system_id=self.target_system,
            timeout=timeout or self.default_timeout,
            cancel=cancel,
        )
        assert isinstance(message, CommandAck)
        return message


CommandLongHandler = Callable[[CommandLong], Awaitable[MavResult] | MavResult]
CommandIntHandler = Callable[[CommandInt], Awaitable[MavResult] | MavResult]


class CommandServer:
    def __init__(
        self,
        session: MavlinkSession,
        on_command_long: CommandLongHandler | None = None,
        on_command_int: CommandIntHandler | None = None,
    ) -> None:
        self.session = session
        self.on_command_long = on_command_long
        self.on_command_int = on_command_int
        self._frame_task = asyncio.create_task(self._listen_frames())

    async def close(self) -> None:
        self._frame_task.cancel()
        try:
            await self._frame_task
        except asyncio.CancelledError:
            pass

    async def _listen_frames(self) -> None:
        async for frame in self.session.frames():
            await self._on_frame(frame)

    async def _on_frame(self, frame: MavlinkFrame) -> None:
        message = frame.message

        if isinstance(message, CommandLong):
            if message.target_system != self.session.system_id:
                return
            if self.on_command_long is not None:
                result = self.on_command_long(message)
            else:
                result = MavResult.MAV_RESULT_ACCEPTED
            if asyncio.iscoroutine(result):
                result = await result
            await self._send_ack(frame, message.command, result)
            return

        if isinstance(message, CommandInt):
            if message.target_system != self.session.system_id:
                return
            if self.on_command_int is not None:
                result = self.on_command_int(message)
            else:
                result = MavResult.MAV_RESULT_ACCEPTED
            if asyncio.iscoroutine(result):
                result = await result
            await self._send_ack(frame, message.command, result)

    async def _send_ack(
        self, request_frame: MavlinkFrame, command: MavCmd, result: MavResult
    ) -> None:
        await self.session.send(
            CommandAck(
                command=command,
                result=result,
                progress=0,
                result_param2=0,
                target_system=request_frame.system_id,
                target_component=request_frame.component_id,
            )
        )
