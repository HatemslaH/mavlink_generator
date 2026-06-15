from __future__ import annotations

from abc import ABC, abstractmethod

from mavlink_message import MavlinkMessage


class MavlinkDialect(ABC):
    @property
    @abstractmethod
    def version(self) -> int:
        ...

    @abstractmethod
    def parse(self, message_id: int, data: bytes) -> MavlinkMessage | None:
        ...

    @abstractmethod
    def crc_extra(self, message_id: int) -> int:
        """Return CRC extra for message_id, or -1 if unsupported."""
