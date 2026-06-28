from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable


class MavlinkCancelledException(Exception):
    def __init__(self, message: str = "Operation cancelled") -> None:
        super().__init__(message)
        self.message = message

    def __str__(self) -> str:
        return f"MavlinkCancelledException: {self.message}"


class MavlinkCancellationToken:
    """Cooperative cancellation token for session waits and protocol flows."""

    def __init__(self) -> None:
        self._cancelled = False
        self._event = asyncio.Event()
        self._callbacks: list[Callable[[], None]] = []

    @property
    def is_cancelled(self) -> bool:
        return self._cancelled

    def on_cancel(self, callback: Callable[[], None]) -> Callable[[], None]:
        """Register a callback fired once when [cancel] is called."""

        if self._cancelled:
            callback()
            return lambda: None

        self._callbacks.append(callback)

        def unsubscribe() -> None:
            try:
                self._callbacks.remove(callback)
            except ValueError:
                pass

        return unsubscribe

    def cancel(self) -> None:
        if self._cancelled:
            return
        self._cancelled = True
        self._event.set()
        for callback in list(self._callbacks):
            callback()

    def throw_if_cancelled(self) -> None:
        if self._cancelled:
            raise MavlinkCancelledException()

    async def wait_cancelled(self) -> None:
        await self._event.wait()

    def dispose(self) -> None:
        self._callbacks.clear()
