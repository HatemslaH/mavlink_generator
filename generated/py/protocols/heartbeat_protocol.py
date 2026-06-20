from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass
from datetime import datetime, timedelta

from mavlink import Heartbeat, MavAutopilot, MavState, MavType
from mavlink_frame import MavlinkFrame

from .mavlink_cancellation import MavlinkCancellationToken, MavlinkCancelledException
from .mavlink_session import MavlinkSession, MavlinkTimeoutException


@dataclass(frozen=True, slots=True)
class MavlinkNode:
    system_id: int
    component_id: int

    def __str__(self) -> str:
        return f"MavlinkNode({self.system_id}:{self.component_id})"


@dataclass(frozen=True, slots=True)
class TrackedHeartbeat:
    node: MavlinkNode
    heartbeat: Heartbeat
    received_at: datetime
    online: bool

    @property
    def age(self) -> timedelta:
        return datetime.now() - self.received_at


class HeartbeatMonitor:
    """Tracks remote HEARTBEAT messages and reports connect / disconnect events."""

    def __init__(
        self,
        session: MavlinkSession,
        timeout: timedelta = timedelta(seconds=5),
        watch: set[MavlinkNode] | None = None,
        watch_system_id: int | None = None,
    ) -> None:
        self.session = session
        self.timeout = timeout
        self.watch = watch
        self.watch_system_id = watch_system_id

        self._states: dict[MavlinkNode, TrackedHeartbeat] = {}
        self._online: dict[MavlinkNode, bool] = {}
        self._heartbeat_subscribers: list[asyncio.Queue[TrackedHeartbeat]] = []
        self._connected_subscribers: list[asyncio.Queue[MavlinkNode]] = []
        self._disconnected_subscribers: list[asyncio.Queue[MavlinkNode]] = []
        self._frame_task: asyncio.Task[None] | None = None
        self._watchdog_task: asyncio.Task[None] | None = None
        self._running = False

    def on_heartbeat(self) -> AsyncIterator[TrackedHeartbeat]:
        return self._iter_events(self._heartbeat_subscribers)

    def on_connected(self) -> AsyncIterator[MavlinkNode]:
        return self._iter_events(self._connected_subscribers)

    def on_disconnected(self) -> AsyncIterator[MavlinkNode]:
        return self._iter_events(self._disconnected_subscribers)

    async def _iter_events(self, subscribers: list[asyncio.Queue]) -> AsyncIterator:
        queue: asyncio.Queue = asyncio.Queue()
        subscribers.append(queue)
        try:
            while self._running:
                item = await queue.get()
                if item is None:
                    break
                yield item
        finally:
            try:
                subscribers.remove(queue)
            except ValueError:
                pass

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        self._frame_task = asyncio.create_task(self._listen_frames())
        self._watchdog_task = asyncio.create_task(self._watchdog_loop())

    async def stop(self) -> None:
        if not self._running:
            return
        self._running = False

        if self._frame_task is not None:
            self._frame_task.cancel()
            try:
                await self._frame_task
            except asyncio.CancelledError:
                pass
            self._frame_task = None

        if self._watchdog_task is not None:
            self._watchdog_task.cancel()
            try:
                await self._watchdog_task
            except asyncio.CancelledError:
                pass
            self._watchdog_task = None

        for subscribers in (
            self._heartbeat_subscribers,
            self._connected_subscribers,
            self._disconnected_subscribers,
        ):
            for queue in list(subscribers):
                queue.put_nowait(None)

    def state_for(self, node: MavlinkNode) -> TrackedHeartbeat | None:
        return self._states.get(node)

    def state_for_ids(self, system_id: int, component_id: int) -> TrackedHeartbeat | None:
        return self.state_for(MavlinkNode(system_id, component_id))

    def is_online(self, node: MavlinkNode) -> bool:
        return self._online.get(node, False)

    def is_online_ids(self, system_id: int, component_id: int) -> bool:
        return self.is_online(MavlinkNode(system_id, component_id))

    @property
    def online_nodes(self) -> list[MavlinkNode]:
        return [node for node, online in self._online.items() if online]

    async def wait_for_vehicle(
        self,
        exclude_system_ids: set[int] | None = None,
        timeout: timedelta = timedelta(seconds=60),
        cancel: MavlinkCancellationToken | None = None,
    ) -> MavlinkNode:
        if cancel is not None:
            cancel.throw_if_cancelled()

        for node in self.online_nodes:
            if exclude_system_ids is None or node.system_id not in exclude_system_ids:
                return node

        loop = asyncio.get_running_loop()
        future: asyncio.Future[MavlinkNode] = loop.create_future()

        async def _listen_connected() -> None:
            async for node in self.on_connected():
                if exclude_system_ids is not None and node.system_id in exclude_system_ids:
                    continue
                if not future.done():
                    future.set_result(node)
                    return

        listen_task = asyncio.create_task(_listen_connected())
        cancel_unsubscribe: Callable | None = None

        if cancel is not None:
            if cancel.is_cancelled:
                listen_task.cancel()
                raise MavlinkCancelledException()

            def _on_cancel() -> None:
                if not future.done():
                    future.set_exception(MavlinkCancelledException())

            cancel_unsubscribe = cancel.on_cancel(_on_cancel)

        try:
            return await asyncio.wait_for(future, timeout.total_seconds())
        except asyncio.TimeoutError as exc:
            raise MavlinkTimeoutException(
                "Timed out waiting for vehicle heartbeat", timeout
            ) from exc
        finally:
            listen_task.cancel()
            try:
                await listen_task
            except asyncio.CancelledError:
                pass
            if cancel_unsubscribe is not None:
                cancel_unsubscribe()

    async def _listen_frames(self) -> None:
        async for frame in self.session.frames():
            self._on_frame(frame)

    async def _watchdog_loop(self) -> None:
        interval = max(self.timeout.total_seconds() / 3, 0.1)
        while self._running:
            await asyncio.sleep(interval)
            self._check_timeouts()

    def _on_frame(self, frame: MavlinkFrame) -> None:
        if not isinstance(frame.message, Heartbeat):
            return

        node = MavlinkNode(frame.system_id, frame.component_id)
        if not self._should_watch(node):
            return

        heartbeat = frame.message
        was_online = self._online.get(node, False)
        now = datetime.now()
        tracked = TrackedHeartbeat(node=node, heartbeat=heartbeat, received_at=now, online=True)

        self._states[node] = tracked
        self._online[node] = True
        self._publish(self._heartbeat_subscribers, tracked)

        if not was_online:
            self._publish(self._connected_subscribers, node)

    def _check_timeouts(self) -> None:
        now = datetime.now()
        for node in list(self._states.keys()):
            state = self._states.get(node)
            if state is None:
                continue

            timed_out = now - state.received_at > self.timeout
            was_online = self._online.get(node, False)

            if timed_out and was_online:
                self._online[node] = False
                self._publish(self._disconnected_subscribers, node)
                self._publish(
                    self._heartbeat_subscribers,
                    TrackedHeartbeat(
                        node=node,
                        heartbeat=state.heartbeat,
                        received_at=state.received_at,
                        online=False,
                    ),
                )

    def _should_watch(self, node: MavlinkNode) -> bool:
        if self.watch is not None:
            return node in self.watch
        if self.watch_system_id is not None:
            return node.system_id == self.watch_system_id
        return True

    def _publish(self, subscribers: list[asyncio.Queue], item) -> None:
        for queue in list(subscribers):
            queue.put_nowait(item)


class HeartbeatPublisher:
    """Periodically sends HEARTBEAT on a [MavlinkSession]."""

    def __init__(
        self,
        session: MavlinkSession,
        heartbeat: Heartbeat,
        interval: timedelta = timedelta(seconds=1),
    ) -> None:
        self.session = session
        self.interval = interval
        self._heartbeat = heartbeat
        self._task: asyncio.Task[None] | None = None
        self._running = False

    @property
    def heartbeat(self) -> Heartbeat:
        return self._heartbeat

    def update_heartbeat(self, heartbeat: Heartbeat) -> None:
        self._heartbeat = heartbeat

    def mutate_heartbeat(self, transform: Callable[[Heartbeat], Heartbeat]) -> None:
        self._heartbeat = transform(self._heartbeat)

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        self._task = asyncio.create_task(self._publish_loop())
        asyncio.create_task(self.send_once())

    def stop(self) -> None:
        self._running = False
        if self._task is not None:
            self._task.cancel()
            self._task = None

    async def send_once(self) -> None:
        await self.session.send(self._heartbeat)

    async def _publish_loop(self) -> None:
        try:
            while self._running:
                await asyncio.sleep(self.interval.total_seconds())
                if self._running:
                    await self.send_once()
        except asyncio.CancelledError:
            raise


class HeartbeatTemplates:
    @staticmethod
    def gcs(mavlink_version: int) -> Heartbeat:
        return Heartbeat(
            custom_mode=0,
            type=MavType.MAV_TYPE_GCS,
            autopilot=MavAutopilot.MAV_AUTOPILOT_INVALID,
            base_mode=0,
            system_status=MavState.MAV_STATE_ACTIVE,
            mavlink_version=mavlink_version,
        )

    @staticmethod
    def autopilot(
        mavlink_version: int,
        mav_type: MavType = MavType.MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot = MavAutopilot.MAV_AUTOPILOT_PX4,
        system_status: MavState = MavState.MAV_STATE_ACTIVE,
        custom_mode: int = 0,
        base_mode: int = 0,
    ) -> Heartbeat:
        return Heartbeat(
            custom_mode=custom_mode,
            type=mav_type,
            autopilot=autopilot,
            base_mode=base_mode,
            system_status=system_status,
            mavlink_version=mavlink_version,
        )

    @staticmethod
    def onboard_api(mavlink_version: int) -> Heartbeat:
        return Heartbeat(
            custom_mode=0,
            type=MavType.MAV_TYPE_ONBOARD_CONTROLLER,
            autopilot=MavAutopilot.MAV_AUTOPILOT_INVALID,
            base_mode=0,
            system_status=MavState.MAV_STATE_ACTIVE,
            mavlink_version=mavlink_version,
        )
