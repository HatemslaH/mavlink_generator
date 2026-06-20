from __future__ import annotations

import struct
from numbers import Real
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from mavlink import MavParamType


class ParamCodec:
    """Byte-wise parameter value encoding per MAVLink parameter protocol."""

    @staticmethod
    def encode_int8(value: int) -> float:
        return ParamCodec._encode_int32(value)

    @staticmethod
    def decode_int8(encoded: float) -> int:
        return ParamCodec.decode_int32(encoded)

    @staticmethod
    def encode_uint8(value: int) -> float:
        return ParamCodec._encode_uint32(value)

    @staticmethod
    def decode_uint8(encoded: float) -> int:
        return ParamCodec.decode_uint32(encoded)

    @staticmethod
    def encode_int16(value: int) -> float:
        return ParamCodec._encode_int32(value)

    @staticmethod
    def decode_int16(encoded: float) -> int:
        return ParamCodec.decode_int32(encoded)

    @staticmethod
    def encode_uint16(value: int) -> float:
        return ParamCodec._encode_uint32(value)

    @staticmethod
    def decode_uint16(encoded: float) -> int:
        return ParamCodec.decode_uint32(encoded)

    @staticmethod
    def encode_int32(value: int) -> float:
        return ParamCodec._encode_int32(value)

    @staticmethod
    def decode_int32(encoded: float) -> int:
        return struct.unpack("<i", struct.pack("<f", encoded))[0]

    @staticmethod
    def encode_uint32(value: int) -> float:
        return ParamCodec._encode_uint32(value)

    @staticmethod
    def decode_uint32(encoded: float) -> int:
        return struct.unpack("<I", struct.pack("<f", encoded))[0]

    @staticmethod
    def encode_float(value: float) -> float:
        return value

    @staticmethod
    def decode_float(encoded: float) -> float:
        return encoded

    @staticmethod
    def encode_value(value: Real, param_type: MavParamType) -> float:
        from mavlink import MavParamType  # noqa: PLC0415

        if param_type == MavParamType.MAV_PARAM_TYPE_UINT8:
            return ParamCodec.encode_uint8(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_INT8:
            return ParamCodec.encode_int8(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_UINT16:
            return ParamCodec.encode_uint16(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_INT16:
            return ParamCodec.encode_int16(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_INT32:
            return ParamCodec.encode_int32(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_UINT32:
            return ParamCodec.encode_uint32(int(value))
        if param_type == MavParamType.MAV_PARAM_TYPE_REAL32:
            return ParamCodec.encode_float(float(value))
        return float(value)

    @staticmethod
    def decode_value(encoded: float, param_type: MavParamType) -> Real:
        from mavlink import MavParamType  # noqa: PLC0415

        if param_type == MavParamType.MAV_PARAM_TYPE_UINT8:
            return ParamCodec.decode_uint8(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_INT8:
            return ParamCodec.decode_int8(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_UINT16:
            return ParamCodec.decode_uint16(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_INT16:
            return ParamCodec.decode_int16(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_INT32:
            return ParamCodec.decode_int32(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_UINT32:
            return ParamCodec.decode_uint32(encoded)
        if param_type == MavParamType.MAV_PARAM_TYPE_REAL32:
            return ParamCodec.decode_float(encoded)
        return encoded

    @staticmethod
    def param_id_from_string(name: str) -> list[int]:
        param_id: list[int] = []
        for code_unit in name.encode("ascii", errors="ignore")[:16]:
            param_id.append(code_unit)
        while len(param_id) < 16:
            param_id.append(0)
        return param_id

    @staticmethod
    def param_id_to_string(param_id: list[int]) -> str:
        end = next((index for index, value in enumerate(param_id) if value == 0), len(param_id))
        return bytes(param_id[:end]).decode("ascii", errors="ignore")

    @staticmethod
    def _encode_int32(value: int) -> float:
        return struct.unpack("<f", struct.pack("<i", value))[0]

    @staticmethod
    def _encode_uint32(value: int) -> float:
        return struct.unpack("<f", struct.pack("<I", value))[0]
