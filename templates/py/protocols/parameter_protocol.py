from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass
from datetime import timedelta
from numbers import Real

from mavlink import (
    MavComponent,
    MavParamType,
    ParamRequestList,
    ParamRequestRead,
    ParamSet,
    ParamValue,
)
from mavlink_frame import MavlinkFrame

from .mavlink_cancellation import MavlinkCancellationToken
from .mavlink_session import MavlinkSession, MavlinkTimeoutException
from .param_codec import ParamCodec


@dataclass(frozen=True, slots=True)
class ParamEntry:
    id: str
    value: Real
    type: MavParamType
    index: int
    count: int

    @classmethod
    def from_param_value(cls, message: ParamValue) -> ParamEntry:
        return cls(
            id=ParamCodec.param_id_to_string(message.param_id),
            value=ParamCodec.decode_value(message.param_value, message.param_type),
            type=message.param_type,
            index=message.param_index,
            count=message.param_count,
        )


ParamProgressCallback = Callable[[ParamEntry, int, int], None]


class ParameterProtocol:
    """GCS-side MAVLink parameter protocol client."""

    def __init__(
        self,
        session: MavlinkSession,
        target_system: int,
        target_component: int,
        idle_timeout: timedelta = timedelta(milliseconds=500),
        request_timeout: timedelta = timedelta(seconds=3),
    ) -> None:
        self.session = session
        self.target_system = target_system
        self.target_component = target_component
        self.idle_timeout = idle_timeout
        self.request_timeout = request_timeout
        self._cache: dict[str, ParamEntry] = {}

    @property
    def cache(self) -> dict[str, ParamEntry]:
        return dict(self._cache)

    def clear_cache(self) -> None:
        self._cache.clear()

    def type_for_name(self, name: str) -> MavParamType | None:
        entry = self._cache.get(name)
        return entry.type if entry is not None else None

    def _remember(self, entry: ParamEntry) -> None:
        self._cache[entry.id] = entry

    @staticmethod
    def _take_next_param(
        inbox: list[ParamValue], seen_indices: set[int]
    ) -> ParamValue | None:
        while inbox:
            next_value = inbox.pop(0)
            if next_value.param_index not in seen_indices:
                return next_value
        return None

    async def _wait_for_next_param(
        self,
        inbox: list[ParamValue],
        seen_indices: set[int],
        timeout: timedelta,
        cancel: MavlinkCancellationToken | None,
    ) -> ParamValue:
        buffered = self._take_next_param(inbox, seen_indices)
        if buffered is not None:
            return buffered

        message = await self.session.wait_for_message(
            predicate=lambda message: (
                isinstance(message, ParamValue)
                and message.param_index not in seen_indices
            ),
            from_system_id=self.target_system,
            from_component_id=self.target_component,
            timeout=timeout,
            cancel=cancel,
        )
        assert isinstance(message, ParamValue)
        return message

    @staticmethod
    def _find_missing_index(
        seen_indices: set[int], expected_count: int
    ) -> int | None:
        for index in range(expected_count):
            if index not in seen_indices:
                return index
        return None

    async def fetch_all(
        self,
        on_progress: ParamProgressCallback | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> list[ParamEntry]:
        entries: list[ParamEntry] = []
        async for entry in self.fetch_all_stream(cancel=cancel):
            entries.append(entry)
            if on_progress is not None:
                on_progress(entry, len(entries), entry.count)
        return entries

    async def fetch_all_stream(
        self, cancel: MavlinkCancellationToken | None = None
    ) -> AsyncIterator[ParamEntry]:
        if cancel is not None:
            cancel.throw_if_cancelled()

        inbox: list[ParamValue] = []
        subscription = self.session.listen_message(
            ParamValue,
            lambda message, _frame: inbox.append(message),
            from_system_id=self.target_system,
            from_component_id=self.target_component,
        )

        try:
            await self.session.send(
                ParamRequestList(
                    target_system=self.target_system,
                    target_component=self.target_component,
                )
            )

            expected_count = -1
            seen_indices: set[int] = set()
            retry_counts: dict[int, int] = {}
            is_retrying = False

            while True:
                if cancel is not None:
                    cancel.throw_if_cancelled()

                param_value = self._take_next_param(inbox, seen_indices)
                if param_value is None:
                    timeout = (
                        self.request_timeout
                        if expected_count == -1 or is_retrying
                        else self.idle_timeout
                    )
                    try:
                        param_value = await self._wait_for_next_param(
                            inbox,
                            seen_indices,
                            timeout,
                            cancel,
                        )
                        is_retrying = False
                    except MavlinkTimeoutException:
                        if expected_count == -1:
                            raise

                        missing_index = self._find_missing_index(
                            seen_indices, expected_count
                        )
                        if missing_index is None:
                            break

                        retries = retry_counts.get(missing_index, 0)
                        if retries >= 3:
                            raise

                        retry_counts[missing_index] = retries + 1
                        is_retrying = True

                        await self.session.send(
                            ParamRequestRead(
                                param_index=missing_index,
                                target_system=self.target_system,
                                target_component=self.target_component,
                                param_id=ParamCodec.param_id_from_string(""),
                            )
                        )
                        continue
                else:
                    is_retrying = False

                if param_value.param_index in seen_indices:
                    continue

                seen_indices.add(param_value.param_index)

                if expected_count == -1:
                    expected_count = param_value.param_count

                entry = ParamEntry.from_param_value(param_value)
                self._remember(entry)
                yield entry

                if len(seen_indices) >= expected_count:
                    break
        finally:
            subscription.cancel()

    async def read_by_name(self, name: str) -> ParamEntry:
        return await self.read(param_id=name)

    async def read_by_index(self, index: int) -> ParamEntry:
        return await self.read(param_index=index)

    async def read(
        self,
        param_id: str | None = None,
        param_index: int = -1,
        cancel: MavlinkCancellationToken | None = None,
    ) -> ParamEntry:
        if param_id is None and param_index < 0:
            raise ValueError("Either param_id or a non-negative param_index is required")

        await self.session.send(
            ParamRequestRead(
                param_index=param_index,
                target_system=self.target_system,
                target_component=self.target_component,
                param_id=ParamCodec.param_id_from_string(param_id or ""),
            )
        )

        value = await self.session.wait_for_message_type(
            ParamValue,
            from_system_id=self.target_system,
            from_component_id=self.target_component,
            timeout=self.request_timeout,
            cancel=cancel,
        )
        entry = ParamEntry.from_param_value(value)
        self._remember(entry)
        return entry

    async def write(
        self,
        name: str,
        value: Real,
        param_type: MavParamType,
        cancel: MavlinkCancellationToken | None = None,
    ) -> ParamEntry:
        await self.session.send(
            ParamSet(
                param_value=ParamCodec.encode_value(value, param_type),
                target_system=self.target_system,
                target_component=self.target_component,
                param_id=ParamCodec.param_id_from_string(name),
                param_type=param_type,
            )
        )

        ack = await self.session.wait_for_message(
            predicate=lambda message: (
                isinstance(message, ParamValue)
                and ParamCodec.param_id_to_string(message.param_id) == name
            ),
            from_system_id=self.target_system,
            from_component_id=self.target_component,
            timeout=self.request_timeout,
            cancel=cancel,
        )
        assert isinstance(ack, ParamValue)
        entry = ParamEntry.from_param_value(ack)
        self._remember(entry)
        return entry

    async def write_by_name(
        self,
        name: str,
        value: Real,
        param_type: MavParamType | None = None,
        cancel: MavlinkCancellationToken | None = None,
    ) -> ParamEntry:
        resolved_type = param_type or self.type_for_name(name) or MavParamType.MAV_PARAM_TYPE_REAL32
        return await self.write(name=name, value=value, param_type=resolved_type, cancel=cancel)


class ParameterServer:
    """Vehicle-side parameter store handler for embedding in autopilot code."""

    def __init__(
        self,
        session: MavlinkSession,
        initial_values: dict[str, tuple[Real, MavParamType]] | None = None,
    ) -> None:
        self.session = session
        self._values: dict[str, tuple[Real, MavParamType]] = dict(initial_values or {})
        self._frame_task = asyncio.create_task(self._listen_frames())

    @property
    def values(self) -> dict[str, tuple[Real, MavParamType]]:
        return dict(self._values)

    async def close(self) -> None:
        self._frame_task.cancel()
        try:
            await self._frame_task
        except asyncio.CancelledError:
            pass

    def set(self, name: str, value: Real, param_type: MavParamType) -> None:
        self._values[name] = (value, param_type)

    async def _listen_frames(self) -> None:
        async for frame in self.session.frames():
            await self._on_frame(frame)

    async def _on_frame(self, frame: MavlinkFrame) -> None:
        message = frame.message

        if isinstance(message, ParamRequestList):
            if (
                message.target_system != self.session.system_id
                and message.target_system != MavComponent.MAV_COMP_ID_ALL
            ):
                return
            await self._broadcast_all()
            return

        if isinstance(message, ParamRequestRead):
            if (
                message.target_system != self.session.system_id
                and message.target_system != MavComponent.MAV_COMP_ID_ALL
            ):
                return
            entry = self._resolve_read(message)
            if entry is not None:
                await self._send_value(entry[0], entry[1], self._index_of(entry[0]))
            return

        if isinstance(message, ParamSet):
            if message.target_system != self.session.system_id:
                return
            name = ParamCodec.param_id_to_string(message.param_id)
            self._values[name] = (
                ParamCodec.decode_value(message.param_value, message.param_type),
                message.param_type,
            )
            await self._send_value(name, self._values[name], self._index_of(name))

    async def _broadcast_all(self) -> None:
        names = list(self._values.keys())
        for index, name in enumerate(names):
            await self._send_value(name, self._values[name], index)

    async def _send_value(
        self, name: str, entry: tuple[Real, MavParamType], index: int
    ) -> None:
        await self.session.send(
            ParamValue(
                param_value=ParamCodec.encode_value(entry[0], entry[1]),
                param_count=len(self._values),
                param_index=index,
                param_id=ParamCodec.param_id_from_string(name),
                param_type=entry[1],
            )
        )

    def _resolve_read(
        self, request: ParamRequestRead
    ) -> tuple[str, tuple[Real, MavParamType]] | None:
        if request.param_index >= 0:
            names = list(self._values.keys())
            if request.param_index >= len(names):
                return None
            name = names[request.param_index]
            return name, self._values[name]

        name = ParamCodec.param_id_to_string(request.param_id)
        entry = self._values.get(name)
        if entry is None:
            return None
        return name, entry

    def _index_of(self, name: str) -> int:
        return list(self._values.keys()).index(name)
