from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator, Awaitable, Callable
from datetime import timedelta
from typing import TypeVar

from mavlink_dialect import MavlinkDialect
from mavlink_frame import MavlinkFrame
from mavlink_message import MavlinkMessage
from mavlink_parser import MavlinkParser
from mavlink_version import MavlinkVersion

from .mavlink_cancellation import MavlinkCancellationToken, MavlinkCancelledException
from .mavlink_link import MavlinkLink

T = TypeVar("T", bound=MavlinkMessage)


class MavlinkTimeoutException(Exception):
    def __init__(self, message: str, timeout: timedelta) -> None:
        super().__init__(message)
        self.message = message
        self.timeout = timeout

    def __str__(self) -> str:
        return f"MavlinkTimeoutException: {self.message} (timeout: {self.timeout})"


class MavlinkMessageSubscription:
    """Handle returned by [MavlinkSession.listen_message]; call [cancel] to unsubscribe."""

    def __init__(self, cancel: Callable[[], None]) -> None:
        self._cancel = cancel
        self._active = True

    @property
    def is_active(self) -> bool:
        return self._active

    def cancel(self) -> None:
        if not self._active:
            return
        self._active = False
        self._cancel()


class _PendingFrameWait:
    def __init__(
        self,
        predicate: Callable[[MavlinkFrame], bool],
        future: asyncio.Future[MavlinkFrame],
        cancel: MavlinkCancellationToken | None,
    ) -> None:
        self.predicate = predicate
        self.future = future
        self.cancel = cancel
        self.cancel_unsubscribe: Callable[[], None] | None = None
        self.timeout_handle: asyncio.TimerHandle | None = None


class _FrameBroadcaster:
    def __init__(self) -> None:
        self._queues: list[asyncio.Queue[MavlinkFrame | None]] = []

    def subscribe(self) -> asyncio.Queue[MavlinkFrame | None]:
        queue: asyncio.Queue[MavlinkFrame | None] = asyncio.Queue()
        self._queues.append(queue)
        return queue

    def unsubscribe(self, queue: asyncio.Queue[MavlinkFrame | None]) -> None:
        try:
            self._queues.remove(queue)
        except ValueError:
            pass

    def publish(self, frame: MavlinkFrame) -> None:
        for queue in list(self._queues):
            queue.put_nowait(frame)

    def close(self) -> None:
        for queue in list(self._queues):
            queue.put_nowait(None)
        self._queues.clear()


class MavlinkSession:
    """Framing, sequencing, and message dispatch over a [MavlinkLink]."""

    _RECENT_FRAME_CAPACITY = 64

    def __init__(
        self,
        dialect: MavlinkDialect,
        link: MavlinkLink,
        system_id: int,
        component_id: int,
        version: MavlinkVersion = MavlinkVersion.V2,
    ) -> None:
        self._dialect = dialect
        self._link = link
        self.system_id = system_id
        self.component_id = component_id
        self.version = version

        self._parser = MavlinkParser(dialect)
        self._broadcaster = _FrameBroadcaster()
        self._pending_waits: list[_PendingFrameWait] = []
        self._recent_frames: list[MavlinkFrame] = []
        self._sequence = 0
        self._closed = False
        self._receive_task: asyncio.Task[None] | None = None
        self._listen_tasks: list[asyncio.Task[None]] = []

    @property
    def dialect(self) -> MavlinkDialect:
        return self._dialect

    def frames(self) -> AsyncIterator[MavlinkFrame]:
        return self._iter_frames()

    async def _iter_frames(self) -> AsyncIterator[MavlinkFrame]:
        queue = self._broadcaster.subscribe()
        try:
            while True:
                frame = await queue.get()
                if frame is None or self._closed:
                    break
                yield frame
        finally:
            self._broadcaster.unsubscribe(queue)

    async def on_message(
        self,
        message_type: type[T],
        from_system_id: int | None = None,
        from_component_id: int | None = None,
    ) -> AsyncIterator[T]:
        async for frame in self.frames():
            if from_system_id is not None and frame.system_id != from_system_id:
                continue
            if from_component_id is not None and frame.component_id != from_component_id:
                continue
            if isinstance(frame.message, message_type):
                yield frame.message

    async def subscribe_message_id(
        self,
        message_id: int,
        from_system_id: int | None = None,
        from_component_id: int | None = None,
    ) -> AsyncIterator[MavlinkMessage]:
        async for frame in self.frames():
            if frame.message.mavlink_message_id != message_id:
                continue
            if from_system_id is not None and frame.system_id != from_system_id:
                continue
            if from_component_id is not None and frame.component_id != from_component_id:
                continue
            yield frame.message

    def listen_message(
        self,
        message_type: type[T],
        on_data: Callable[[T, MavlinkFrame], Awaitable[None] | None],
        from_system_id: int | None = None,
        from_component_id: int | None = None,
    ) -> MavlinkMessageSubscription:
        self._ensure_receive_task()

        async def _listen() -> None:
            async for frame in self.frames():
                if from_system_id is not None and frame.system_id != from_system_id:
                    continue
                if from_component_id is not None and frame.component_id != from_component_id:
                    continue
                if isinstance(frame.message, message_type):
                    result = on_data(frame.message, frame)
                    if asyncio.iscoroutine(result):
                        await result

        task = asyncio.create_task(_listen())
        self._listen_tasks.append(task)

        def _cancel() -> None:
            task.cancel()
            try:
                self._listen_tasks.remove(task)
            except ValueError:
                pass

        return MavlinkMessageSubscription(_cancel)

    async def send(self, message: MavlinkMessage) -> None:
        if self._closed:
            raise RuntimeError("MavlinkSession is closed")

        self._ensure_receive_task()

        if self.version == MavlinkVersion.V2:
            frame = MavlinkFrame.v2(
                self._sequence & 0xFF,
                self.system_id,
                self.component_id,
                message,
            )
        else:
            frame = MavlinkFrame.v1(
                self._sequence & 0xFF,
                self.system_id,
                self.component_id,
                message,
            )
        self._sequence += 1
        await self._link.send(frame.serialize())

    async def wait_for_frame(
        self,
        predicate: Callable[[MavlinkFrame], bool],
        timeout: timedelta = timedelta(seconds=5),
        cancel: MavlinkCancellationToken | None = None,
    ) -> MavlinkFrame:
        if cancel is not None:
            cancel.throw_if_cancelled()

        self._ensure_receive_task()

        loop = asyncio.get_running_loop()
        future: asyncio.Future[MavlinkFrame] = loop.create_future()
        wait = _PendingFrameWait(predicate, future, cancel)

        def _on_timeout() -> None:
            self._remove_wait(wait)
            if not future.done():
                future.set_exception(
                    MavlinkTimeoutException("Timed out waiting for frame", timeout)
                )

        wait.timeout_handle = loop.call_later(timeout.total_seconds(), _on_timeout)

        if cancel is not None:
            if cancel.is_cancelled:
                self._remove_wait(wait)
                raise MavlinkCancelledException()
            wait.cancel_unsubscribe = cancel.on_cancel(
                lambda: self._complete_wait_cancelled(wait)
            )

        self._pending_waits.append(wait)

        for frame in list(self._recent_frames):
            if not predicate(frame):
                continue
            self._recent_frames.remove(frame)
            self._complete_wait_success(wait, frame)
            return await future

        try:
            return await future
        finally:
            self._remove_wait(wait)

    async def wait_for_message(
        self,
        predicate: Callable[[MavlinkMessage], bool],
        from_system_id: int | None = None,
        from_component_id: int | None = None,
        timeout: timedelta = timedelta(seconds=5),
        cancel: MavlinkCancellationToken | None = None,
    ) -> MavlinkMessage:
        frame = await self.wait_for_frame(
            predicate=lambda frame: (
                (from_system_id is None or frame.system_id == from_system_id)
                and (from_component_id is None or frame.component_id == from_component_id)
                and predicate(frame.message)
            ),
            timeout=timeout,
            cancel=cancel,
        )
        return frame.message

    async def wait_for_message_type(
        self,
        message_type: type[T],
        from_system_id: int | None = None,
        from_component_id: int | None = None,
        timeout: timedelta = timedelta(seconds=5),
        cancel: MavlinkCancellationToken | None = None,
    ) -> T:
        message = await self.wait_for_message(
            predicate=lambda message: isinstance(message, message_type),
            from_system_id=from_system_id,
            from_component_id=from_component_id,
            timeout=timeout,
            cancel=cancel,
        )
        return message  # type: ignore[return-value]

    async def close(self) -> None:
        if self._closed:
            return
        self._closed = True

        for wait in list(self._pending_waits):
            self._complete_wait_closed(wait)

        self._pending_waits.clear()
        self._broadcaster.close()

        if self._receive_task is not None:
            self._receive_task.cancel()
            try:
                await self._receive_task
            except asyncio.CancelledError:
                pass
            self._receive_task = None

        for task in list(self._listen_tasks):
            task.cancel()
        self._listen_tasks.clear()

        await self._link.close()

    def _ensure_receive_task(self) -> None:
        if self._receive_task is None and not self._closed:
            self._receive_task = asyncio.create_task(self._receive_loop())

    async def _receive_loop(self) -> None:
        try:
            async for data in self._link.receive():
                if self._closed:
                    break
                self._parser.parse(data)
                while self._parser.frames:
                    frame = self._parser.frames.pop(0)
                    await self._on_frame(frame)
        except asyncio.CancelledError:
            raise
        except Exception:
            if not self._closed:
                raise

    async def _on_frame(self, frame: MavlinkFrame) -> None:
        if self._closed:
            return

        self._broadcaster.publish(frame)
        self._recent_frames.append(frame)
        if len(self._recent_frames) > self._RECENT_FRAME_CAPACITY:
            self._recent_frames.pop(0)

        for wait in list(self._pending_waits):
            if not wait.predicate(frame):
                continue
            if frame in self._recent_frames:
                self._recent_frames.remove(frame)
            self._complete_wait_success(wait, frame)
            break

    def _complete_wait_success(self, wait: _PendingFrameWait, frame: MavlinkFrame) -> None:
        self._remove_wait(wait)
        if not wait.future.done():
            wait.future.set_result(frame)

    def _complete_wait_cancelled(self, wait: _PendingFrameWait) -> None:
        self._remove_wait(wait)
        if not wait.future.done():
            wait.future.set_exception(MavlinkCancelledException())

    def _complete_wait_closed(self, wait: _PendingFrameWait) -> None:
        self._remove_wait(wait)
        if not wait.future.done():
            wait.future.set_exception(RuntimeError("MavlinkSession is closed"))

    def _remove_wait(self, wait: _PendingFrameWait) -> None:
        if wait.timeout_handle is not None:
            wait.timeout_handle.cancel()
            wait.timeout_handle = None
        if wait.cancel_unsubscribe is not None:
            wait.cancel_unsubscribe()
            wait.cancel_unsubscribe = None
        try:
            self._pending_waits.remove(wait)
        except ValueError:
            pass
