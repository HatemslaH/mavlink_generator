from __future__ import annotations

from crc import CrcX25
from mavlink_message import MavlinkMessage
from mavlink_version import MavlinkVersion


class MavlinkFrame:
    MAVLINK_STX_V1 = 0xFE
    MAVLINK_STX_V2 = 0xFD

    def __init__(
        self,
        version: MavlinkVersion,
        sequence: int,
        system_id: int,
        component_id: int,
        message: MavlinkMessage,
    ) -> None:
        self.version = version
        self.sequence = sequence
        self.system_id = system_id
        self.component_id = component_id
        self.message = message

    @classmethod
    def v1(
        cls,
        sequence: int,
        system_id: int,
        component_id: int,
        message: MavlinkMessage,
    ) -> MavlinkFrame:
        return cls(MavlinkVersion.V1, sequence, system_id, component_id, message)

    @classmethod
    def v2(
        cls,
        sequence: int,
        system_id: int,
        component_id: int,
        message: MavlinkMessage,
    ) -> MavlinkFrame:
        return cls(MavlinkVersion.V2, sequence, system_id, component_id, message)

    def serialize(self) -> bytes:
        if self.version == MavlinkVersion.V1:
            return self._serialize_v1()
        return self._serialize_v2()

    def _serialize_v1(self) -> bytes:
        payload = self.message.serialize()
        payload_length = len(payload)
        frame = bytearray(8 + payload_length)
        frame[0] = self.MAVLINK_STX_V1
        frame[1] = payload_length
        frame[2] = self.sequence
        frame[3] = self.system_id
        frame[4] = self.component_id
        frame[5] = self.message.mavlink_message_id

        crc = CrcX25()
        crc.accumulate(payload_length)
        crc.accumulate(self.sequence)
        crc.accumulate(self.system_id)
        crc.accumulate(self.component_id)
        crc.accumulate(self.message.mavlink_message_id)

        for i in range(payload_length):
            frame[6 + i] = payload[i]
            crc.accumulate(payload[i])
        crc.accumulate(self.message.mavlink_crc_extra)

        frame[-2] = crc.crc & 0xFF
        frame[-1] = (crc.crc >> 8) & 0xFF
        return bytes(frame)

    def _serialize_v2(self) -> bytes:
        incompatibility_flags = 0
        compatibility_flags = 0
        payload = self._trim_trailing_zeros(self.message.serialize())
        payload_length = len(payload)
        message_id = self.message.mavlink_message_id
        message_id_bytes = [
            message_id & 0xFF,
            (message_id >> 8) & 0xFF,
            (message_id >> 16) & 0xFF,
        ]

        frame = bytearray(12 + payload_length)
        frame[0] = self.MAVLINK_STX_V2
        frame[1] = payload_length
        frame[2] = incompatibility_flags
        frame[3] = compatibility_flags
        frame[4] = self.sequence
        frame[5] = self.system_id
        frame[6] = self.component_id
        frame[7] = message_id_bytes[0]
        frame[8] = message_id_bytes[1]
        frame[9] = message_id_bytes[2]

        crc = CrcX25()
        crc.accumulate(payload_length)
        crc.accumulate(incompatibility_flags)
        crc.accumulate(compatibility_flags)
        crc.accumulate(self.sequence)
        crc.accumulate(self.system_id)
        crc.accumulate(self.component_id)
        for byte in message_id_bytes:
            crc.accumulate(byte)

        for i in range(payload_length):
            frame[10 + i] = payload[i]
            crc.accumulate(payload[i])
        crc.accumulate(self.message.mavlink_crc_extra)

        frame[-2] = crc.crc & 0xFF
        frame[-1] = (crc.crc >> 8) & 0xFF
        return bytes(frame)

    @staticmethod
    def _trim_trailing_zeros(payload: bytes) -> bytes:
        trimmed_length = len(payload)
        while trimmed_length > 0 and payload[trimmed_length - 1] == 0:
            trimmed_length -= 1
        if trimmed_length == len(payload):
            return payload
        return payload[:trimmed_length]
