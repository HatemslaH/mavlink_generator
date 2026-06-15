from __future__ import annotations

from collections.abc import Callable
from enum import Enum, auto

from crc import CrcX25
from mavlink_dialect import MavlinkDialect
from mavlink_frame import MavlinkFrame
from mavlink_version import MavlinkVersion


class _ParserState(Enum):
    INIT = auto()
    WAIT_PAYLOAD_LENGTH = auto()
    WAIT_INCOMPATIBILITY_FLAGS = auto()
    WAIT_COMPATIBILITY_FLAGS = auto()
    WAIT_PACKET_SEQUENCE = auto()
    WAIT_SYSTEM_ID = auto()
    WAIT_COMPONENT_ID = auto()
    WAIT_MESSAGE_ID_LOW = auto()
    WAIT_MESSAGE_ID_MIDDLE = auto()
    WAIT_MESSAGE_ID_HIGH = auto()
    WAIT_PAYLOAD_END = auto()
    WAIT_CRC_LOW_BYTE = auto()
    WAIT_CRC_HIGH_BYTE = auto()
    WAIT_SIGNATURE_TRAILER = auto()


class MavlinkParser:
    _MAVLINK_MAXIMUM_PAYLOAD_SIZE = 255
    _MAVLINK_IFLAG_SIGNED = 0x01
    _MAVLINK_SIGNATURE_LENGTH = 13

    def __init__(
        self,
        dialect: MavlinkDialect,
        on_signed_packet_dropped: Callable[[int], None] | None = None,
    ) -> None:
        self._dialect = dialect
        self.on_signed_packet_dropped = on_signed_packet_dropped
        self._frames: list[MavlinkFrame] = []
        self._reset_context()
        self._state = _ParserState.INIT

    @property
    def frames(self) -> list[MavlinkFrame]:
        return self._frames

    def _reset_context(self) -> None:
        self._version = MavlinkVersion.V1
        self._payload_length = -1
        self._incompatibility_flags = -1
        self._compatibility_flags = -1
        self._sequence = -1
        self._system_id = -1
        self._component_id = -1
        self._message_id_low = -1
        self._message_id_middle = -1
        self._message_id_high = -1
        self._message_id = -1
        self._payload = bytearray(self._MAVLINK_MAXIMUM_PAYLOAD_SIZE)
        self._payload_cursor = -1
        self._crc_low_byte = -1
        self._crc_high_byte = -1
        self._signature_bytes_remaining = 0

    def _check_crc(self) -> bool:
        if self._version == MavlinkVersion.V1:
            header = [
                self._payload_length,
                self._sequence,
                self._system_id,
                self._component_id,
                self._message_id,
            ]
        else:
            header = [
                self._payload_length,
                self._incompatibility_flags,
                self._compatibility_flags,
                self._sequence,
                self._system_id,
                self._component_id,
                self._message_id_low,
                self._message_id_middle,
                self._message_id_high,
            ]

        crc = CrcX25()
        for value in header:
            crc.accumulate(value & 0xFF)
        for i in range(self._payload_length):
            crc.accumulate(self._payload[i] & 0xFF)

        crc_ext = self._dialect.crc_extra(self._message_id)
        if crc_ext == -1:
            return False
        crc.accumulate(crc_ext)
        return crc.crc == ((self._crc_high_byte << 8) ^ self._crc_low_byte)

    def parse(self, data: bytes) -> None:
        for byte in data:
            self._parse_byte(byte)

    def _parse_byte(self, byte: int) -> None:
        if self._state == _ParserState.INIT:
            if byte == MavlinkFrame.MAVLINK_STX_V1:
                self._version = MavlinkVersion.V1
                self._state = _ParserState.WAIT_PAYLOAD_LENGTH
            elif byte == MavlinkFrame.MAVLINK_STX_V2:
                self._version = MavlinkVersion.V2
                self._state = _ParserState.WAIT_PAYLOAD_LENGTH
            return

        if self._state == _ParserState.WAIT_PAYLOAD_LENGTH:
            self._payload_length = byte
            if self._version == MavlinkVersion.V1:
                self._state = _ParserState.WAIT_PACKET_SEQUENCE
            else:
                self._state = _ParserState.WAIT_INCOMPATIBILITY_FLAGS
            return

        if self._state == _ParserState.WAIT_INCOMPATIBILITY_FLAGS:
            self._incompatibility_flags = byte
            self._state = _ParserState.WAIT_COMPATIBILITY_FLAGS
            return

        if self._state == _ParserState.WAIT_COMPATIBILITY_FLAGS:
            self._compatibility_flags = byte
            self._state = _ParserState.WAIT_PACKET_SEQUENCE
            return

        if self._state == _ParserState.WAIT_PACKET_SEQUENCE:
            self._sequence = byte
            self._state = _ParserState.WAIT_SYSTEM_ID
            return

        if self._state == _ParserState.WAIT_SYSTEM_ID:
            self._system_id = byte
            self._state = _ParserState.WAIT_COMPONENT_ID
            return

        if self._state == _ParserState.WAIT_COMPONENT_ID:
            self._component_id = byte
            if self._version == MavlinkVersion.V1:
                self._state = _ParserState.WAIT_MESSAGE_ID_HIGH
            else:
                self._state = _ParserState.WAIT_MESSAGE_ID_LOW
            return

        if self._state == _ParserState.WAIT_MESSAGE_ID_LOW:
            self._message_id_low = byte
            self._state = _ParserState.WAIT_MESSAGE_ID_MIDDLE
            return

        if self._state == _ParserState.WAIT_MESSAGE_ID_MIDDLE:
            self._message_id_middle = byte
            self._state = _ParserState.WAIT_MESSAGE_ID_HIGH
            return

        if self._state == _ParserState.WAIT_MESSAGE_ID_HIGH:
            if self._version == MavlinkVersion.V1:
                self._message_id = byte
            else:
                self._message_id_high = byte
                self._message_id = (
                    (self._message_id_high << 16)
                    ^ (self._message_id_middle << 8)
                    ^ self._message_id_low
                )
            if self._payload_length == 0:
                self._state = _ParserState.WAIT_CRC_LOW_BYTE
            else:
                self._state = _ParserState.WAIT_PAYLOAD_END
                self._payload_cursor = 0
            return

        if self._state == _ParserState.WAIT_PAYLOAD_END:
            if self._payload_cursor < self._payload_length:
                self._payload[self._payload_cursor] = byte
                self._payload_cursor += 1
            if self._payload_cursor == self._payload_length:
                self._state = _ParserState.WAIT_CRC_LOW_BYTE
            return

        if self._state == _ParserState.WAIT_CRC_LOW_BYTE:
            self._crc_low_byte = byte
            self._state = _ParserState.WAIT_CRC_HIGH_BYTE
            return

        if self._state == _ParserState.WAIT_CRC_HIGH_BYTE:
            self._crc_high_byte = byte
            if (
                self._version == MavlinkVersion.V2
                and (self._incompatibility_flags & self._MAVLINK_IFLAG_SIGNED) != 0
            ):
                if self.on_signed_packet_dropped is not None:
                    self.on_signed_packet_dropped(self._message_id)
                self._signature_bytes_remaining = self._MAVLINK_SIGNATURE_LENGTH
                self._state = _ParserState.WAIT_SIGNATURE_TRAILER
                return

            self._add_mavlink_frame()
            self._reset_context()
            self._state = _ParserState.INIT
            return

        if self._state == _ParserState.WAIT_SIGNATURE_TRAILER:
            self._signature_bytes_remaining -= 1
            if self._signature_bytes_remaining == 0:
                self._reset_context()
                self._state = _ParserState.INIT

    def _add_mavlink_frame(self) -> bool:
        if not self._check_crc():
            return False

        message = self._dialect.parse(
            self._message_id,
            bytes(self._payload[: self._payload_length]),
        )
        if message is None:
            return False

        frame = MavlinkFrame(
            self._version,
            self._sequence,
            self._system_id,
            self._component_id,
            message,
        )
        self._frames.append(frame)
        return True
