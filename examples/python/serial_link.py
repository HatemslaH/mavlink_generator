from __future__ import annotations

import asyncio
from collections.abc import AsyncIterator

import serial


class SerialMavlinkLink:
    """MavlinkLink implementation over a pyserial port (cross-platform)."""

    def __init__(self, port: serial.Serial) -> None:
        self._port = port
        self._queue: asyncio.Queue[bytes | None] = asyncio.Queue()
        self._closed = False
        self._read_task: asyncio.Task[None] | None = None

    @classmethod
    def open(cls, port_name: str, baud_rate: int = 57600) -> SerialMavlinkLink:
        port = serial.Serial(
            port=port_name,
            baudrate=baud_rate,
            bytesize=serial.EIGHTBITS,
            parity=serial.PARITY_NONE,
            stopbits=serial.STOPBITS_ONE,
            timeout=0.05,
            write_timeout=None,
        )
        port.dtr = True
        port.rts = True
        return cls(port)

    def _ensure_read_task(self) -> None:
        if self._read_task is None and not self._closed:
            self._read_task = asyncio.create_task(self._read_loop())

    async def _read_loop(self) -> None:
        loop = asyncio.get_running_loop()
        while not self._closed and self._port.is_open:
            try:
                data = await loop.run_in_executor(None, self._port.read, 4096)
            except Exception:
                if not self._closed:
                    raise
                break
            if self._closed:
                break
            if data:
                await self._queue.put(bytes(data))

    async def receive(self) -> AsyncIterator[bytes]:
        self._ensure_read_task()
        while not self._closed:
            data = await self._queue.get()
            if data is None:
                break
            yield data

    async def send(self, data: bytes) -> None:
        if self._closed:
            raise RuntimeError("SerialMavlinkLink is closed")
        loop = asyncio.get_running_loop()
        written = await loop.run_in_executor(None, self._port.write, bytes(data))
        if written != len(data):
            raise RuntimeError(f"Serial write failed on {self._port.port}")

    async def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        await self._queue.put(None)
        if self._read_task is not None:
            self._read_task.cancel()
            try:
                await self._read_task
            except asyncio.CancelledError:
                pass
            self._read_task = None
        if self._port.is_open:
            loop = asyncio.get_running_loop()
            await loop.run_in_executor(None, self._port.close)
