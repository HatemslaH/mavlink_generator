from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator
from typing import Protocol, runtime_checkable


@runtime_checkable
class MavlinkLink(Protocol):
    """Transport-agnostic MAVLink byte stream."""

    async def send(self, data: bytes) -> None:
        """Send raw MAVLink frame bytes to the remote peer."""
        ...

    def receive(self) -> AsyncIterator[bytes]:
        """Incoming raw bytes from the remote peer."""
        ...

    async def close(self) -> None:
        """Release link resources. Default implementation is a no-op."""
        ...


class _VirtualMavlinkEndpoint:
    def __init__(self, bus: VirtualMavlinkBus) -> None:
        self._bus = bus
        self._queue: asyncio.Queue[bytes | None] = asyncio.Queue()
        self._closed = False

    async def send(self, data: bytes) -> None:
        if self._closed:
            raise RuntimeError("VirtualMavlinkEndpoint is closed")
        self._bus._deliver(bytes(data), self)

    async def receive(self) -> AsyncIterator[bytes]:
        while not self._closed:
            data = await self._queue.get()
            if data is None:
                break
            yield data

    async def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        await self._queue.put(None)
        self._bus._endpoints.remove(self)


class VirtualMavlinkBus:
    """In-memory link for tests and virtual examples."""

    def __init__(self) -> None:
        self._endpoints: list[_VirtualMavlinkEndpoint] = []

    def create_endpoint(self) -> MavlinkLink:
        endpoint = _VirtualMavlinkEndpoint(self)
        self._endpoints.append(endpoint)
        return endpoint

    def _deliver(self, data: bytes, sender: _VirtualMavlinkEndpoint) -> None:
        for endpoint in self._endpoints:
            if endpoint is not sender and not endpoint._closed:
                endpoint._queue.put_nowait(data)

    async def close_all(self) -> None:
        endpoints = list(self._endpoints)
        for endpoint in endpoints:
            await endpoint.close()
