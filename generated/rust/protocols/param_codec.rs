//! Byte-wise parameter value encoding per MAVLink parameter protocol.

use crate::MavParamType;

/// Byte-wise parameter value encoding per MAVLink parameter protocol.
///
/// See <https://mavlink.io/en/services/parameter.html>
pub struct ParamCodec;

impl ParamCodec {
    pub fn encode_int8(value: i8) -> f32 {
        Self::encode_int32(value as i32)
    }

    pub fn decode_int8(encoded: f32) -> i8 {
        Self::decode_int32(encoded) as i8
    }

    pub fn encode_uint8(value: u8) -> f32 {
        Self::encode_uint32(value as u32)
    }

    pub fn decode_uint8(encoded: f32) -> u8 {
        Self::decode_uint32(encoded) as u8
    }

    pub fn encode_int16(value: i16) -> f32 {
        Self::encode_int32(value as i32)
    }

    pub fn decode_int16(encoded: f32) -> i16 {
        Self::decode_int32(encoded) as i16
    }

    pub fn encode_uint16(value: u16) -> f32 {
        Self::encode_uint32(value as u32)
    }

    pub fn decode_uint16(encoded: f32) -> u16 {
        Self::decode_uint32(encoded) as u16
    }

    pub fn encode_int32(value: i32) -> f32 {
        f32::from_bits((value as u32).to_le())
    }

    pub fn decode_int32(encoded: f32) -> i32 {
        encoded.to_bits() as i32
    }

    pub fn encode_uint32(value: u32) -> f32 {
        f32::from_bits(value.to_le())
    }

    pub fn decode_uint32(encoded: f32) -> u32 {
        encoded.to_bits()
    }

    pub fn encode_float(value: f32) -> f32 {
        value
    }

    pub fn decode_float(encoded: f32) -> f32 {
        encoded
    }

    pub fn encode_value(value: f64, param_type: MavParamType) -> f32 {
        match param_type {
            MavParamType::MAV_PARAM_TYPE_UINT8 => Self::encode_uint8(value as u8),
            MavParamType::MAV_PARAM_TYPE_INT8 => Self::encode_int8(value as i8),
            MavParamType::MAV_PARAM_TYPE_UINT16 => Self::encode_uint16(value as u16),
            MavParamType::MAV_PARAM_TYPE_INT16 => Self::encode_int16(value as i16),
            MavParamType::MAV_PARAM_TYPE_INT32 => Self::encode_int32(value as i32),
            MavParamType::MAV_PARAM_TYPE_UINT32 => Self::encode_uint32(value as u32),
            MavParamType::MAV_PARAM_TYPE_REAL32 => Self::encode_float(value as f32),
            _ => value as f32,
        }
    }

    pub fn decode_value(encoded: f32, param_type: MavParamType) -> f64 {
        match param_type {
            MavParamType::MAV_PARAM_TYPE_UINT8 => f64::from(Self::decode_uint8(encoded)),
            MavParamType::MAV_PARAM_TYPE_INT8 => f64::from(Self::decode_int8(encoded)),
            MavParamType::MAV_PARAM_TYPE_UINT16 => f64::from(Self::decode_uint16(encoded)),
            MavParamType::MAV_PARAM_TYPE_INT16 => f64::from(Self::decode_int16(encoded)),
            MavParamType::MAV_PARAM_TYPE_INT32 => f64::from(Self::decode_int32(encoded)),
            MavParamType::MAV_PARAM_TYPE_UINT32 => f64::from(Self::decode_uint32(encoded)),
            MavParamType::MAV_PARAM_TYPE_REAL32 => f64::from(Self::decode_float(encoded)),
            _ => f64::from(encoded),
        }
    }

    /// Encode a short ASCII parameter name into a MAVLink `char[16]` field.
    pub fn param_id_from_string(name: &str) -> [u8; 16] {
        let mut id = [0u8; 16];
        for (index, byte) in name.bytes().take(16).enumerate() {
            id[index] = byte;
        }
        id
    }

    /// Decode a MAVLink `char[16]` parameter id to a string.
    pub fn param_id_to_string(id: &[u8; 16]) -> String {
        let end = id.iter().position(|&c| c == 0).unwrap_or(id.len());
        String::from_utf8_lossy(&id[..end]).into_owned()
    }
}
