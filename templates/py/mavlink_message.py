from __future__ import annotations

import struct
from abc import ABC, abstractmethod


class MavlinkMessage(ABC):
    @property
    @abstractmethod
    def mavlink_message_id(self) -> int:
        ...

    @property
    @abstractmethod
    def mavlink_crc_extra(self) -> int:
        ...

    @abstractmethod
    def serialize(self) -> bytes:
        ...

    @staticmethod
    def _endian() -> str:
        return "<"

    @classmethod
    def _get_int8(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}b", data, offset)[0]

    @classmethod
    def _get_uint8(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}B", data, offset)[0]

    @classmethod
    def _get_int16(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}h", data, offset)[0]

    @classmethod
    def _get_uint16(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}H", data, offset)[0]

    @classmethod
    def _get_int32(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}i", data, offset)[0]

    @classmethod
    def _get_uint32(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}I", data, offset)[0]

    @classmethod
    def _get_int64(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}q", data, offset)[0]

    @classmethod
    def _get_uint64(cls, data: bytes, offset: int) -> int:
        return struct.unpack_from(f"{cls._endian()}Q", data, offset)[0]

    @classmethod
    def _get_float32(cls, data: bytes, offset: int) -> float:
        return struct.unpack_from(f"{cls._endian()}f", data, offset)[0]

    @classmethod
    def _get_float64(cls, data: bytes, offset: int) -> float:
        return struct.unpack_from(f"{cls._endian()}d", data, offset)[0]

    @classmethod
    def _get_int8_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_int8(data, offset + i) for i in range(length)]

    @classmethod
    def _get_uint8_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_uint8(data, offset + i) for i in range(length)]

    @classmethod
    def _get_int16_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_int16(data, offset + i * 2) for i in range(length)]

    @classmethod
    def _get_uint16_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_uint16(data, offset + i * 2) for i in range(length)]

    @classmethod
    def _get_int32_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_int32(data, offset + i * 4) for i in range(length)]

    @classmethod
    def _get_uint32_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_uint32(data, offset + i * 4) for i in range(length)]

    @classmethod
    def _get_int64_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_int64(data, offset + i * 8) for i in range(length)]

    @classmethod
    def _get_uint64_list(cls, data: bytes, offset: int, length: int) -> list[int]:
        return [cls._get_uint64(data, offset + i * 8) for i in range(length)]

    @classmethod
    def _get_float32_list(cls, data: bytes, offset: int, length: int) -> list[float]:
        return [cls._get_float32(data, offset + i * 4) for i in range(length)]

    @classmethod
    def _get_float64_list(cls, data: bytes, offset: int, length: int) -> list[float]:
        return [cls._get_float64(data, offset + i * 8) for i in range(length)]

    @classmethod
    def _set_int8(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}b", data, offset, value)

    @classmethod
    def _set_uint8(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}B", data, offset, value)

    @classmethod
    def _set_int16(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}h", data, offset, value)

    @classmethod
    def _set_uint16(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}H", data, offset, value)

    @classmethod
    def _set_int32(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}i", data, offset, value)

    @classmethod
    def _set_uint32(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}I", data, offset, value)

    @classmethod
    def _set_int64(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}q", data, offset, value)

    @classmethod
    def _set_uint64(cls, data: bytearray, offset: int, value: int) -> None:
        struct.pack_into(f"{cls._endian()}Q", data, offset, value)

    @classmethod
    def _set_float32(cls, data: bytearray, offset: int, value: float) -> None:
        struct.pack_into(f"{cls._endian()}f", data, offset, value)

    @classmethod
    def _set_float64(cls, data: bytearray, offset: int, value: float) -> None:
        struct.pack_into(f"{cls._endian()}d", data, offset, value)

    @classmethod
    def _set_int8_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_int8(data, offset + i, value)

    @classmethod
    def _set_uint8_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_uint8(data, offset + i, value)

    @classmethod
    def _set_int16_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_int16(data, offset + i * 2, value)

    @classmethod
    def _set_uint16_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_uint16(data, offset + i * 2, value)

    @classmethod
    def _set_int32_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_int32(data, offset + i * 4, value)

    @classmethod
    def _set_uint32_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_uint32(data, offset + i * 4, value)

    @classmethod
    def _set_int64_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_int64(data, offset + i * 8, value)

    @classmethod
    def _set_uint64_list(cls, data: bytearray, offset: int, values: list[int]) -> None:
        for i, value in enumerate(values):
            cls._set_uint64(data, offset + i * 8, value)

    @classmethod
    def _set_float32_list(cls, data: bytearray, offset: int, values: list[float]) -> None:
        for i, value in enumerate(values):
            cls._set_float32(data, offset + i * 4, value)

    @classmethod
    def _set_float64_list(cls, data: bytearray, offset: int, values: list[float]) -> None:
        for i, value in enumerate(values):
            cls._set_float64(data, offset + i * 8, value)
