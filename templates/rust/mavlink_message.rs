pub trait MavlinkMessage: std::fmt::Debug + std::any::Any {
    fn mavlink_message_id(&self) -> u32;

    fn mavlink_crc_extra(&self) -> u8;

    fn serialize(&self) -> Vec<u8>;
}

pub(crate) fn pad_payload(data: &[u8], length: usize) -> Vec<u8> {
    if data.len() >= length {
        return data.to_vec();
    }
    let mut padded = data.to_vec();
    padded.resize(length, 0);
    padded
}

pub(crate) fn get_i8(data: &[u8], offset: usize) -> i8 {
    data[offset] as i8
}

pub(crate) fn get_u8(data: &[u8], offset: usize) -> u8 {
    data[offset]
}

pub(crate) fn get_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

pub(crate) fn get_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

pub(crate) fn get_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub(crate) fn get_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub(crate) fn get_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

pub(crate) fn get_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

pub(crate) fn get_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub(crate) fn get_f64(data: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

pub(crate) fn get_i8_array<const N: usize>(data: &[u8], offset: usize) -> [i8; N] {
    let mut out = [0i8; N];
    for (i, byte) in data[offset..offset + N].iter().enumerate() {
        out[i] = *byte as i8;
    }
    out
}

pub(crate) fn get_u8_array<const N: usize>(data: &[u8], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&data[offset..offset + N]);
    out
}

pub(crate) fn get_i16_array<const N: usize>(data: &[u8], offset: usize) -> [i16; N] {
    let mut out = [0i16; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_i16(data, offset + i * 2);
    }
    out
}

pub(crate) fn get_u16_array<const N: usize>(data: &[u8], offset: usize) -> [u16; N] {
    let mut out = [0u16; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_u16(data, offset + i * 2);
    }
    out
}

pub(crate) fn get_i32_array<const N: usize>(data: &[u8], offset: usize) -> [i32; N] {
    let mut out = [0i32; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_i32(data, offset + i * 4);
    }
    out
}

pub(crate) fn get_u32_array<const N: usize>(data: &[u8], offset: usize) -> [u32; N] {
    let mut out = [0u32; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_u32(data, offset + i * 4);
    }
    out
}

pub(crate) fn get_i64_array<const N: usize>(data: &[u8], offset: usize) -> [i64; N] {
    let mut out = [0i64; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_i64(data, offset + i * 8);
    }
    out
}

pub(crate) fn get_u64_array<const N: usize>(data: &[u8], offset: usize) -> [u64; N] {
    let mut out = [0u64; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_u64(data, offset + i * 8);
    }
    out
}

pub(crate) fn get_f32_array<const N: usize>(data: &[u8], offset: usize) -> [f32; N] {
    let mut out = [0f32; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_f32(data, offset + i * 4);
    }
    out
}

pub(crate) fn get_f64_array<const N: usize>(data: &[u8], offset: usize) -> [f64; N] {
    let mut out = [0f64; N];
    for (i, item) in out.iter_mut().enumerate() {
        *item = get_f64(data, offset + i * 8);
    }
    out
}

pub(crate) fn set_i8(data: &mut [u8], offset: usize, value: i8) {
    data[offset] = value as u8;
}

pub(crate) fn set_u8(data: &mut [u8], offset: usize, value: u8) {
    data[offset] = value;
}

pub(crate) fn set_i16(data: &mut [u8], offset: usize, value: i16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_i64(data: &mut [u8], offset: usize, value: i64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_f32(data: &mut [u8], offset: usize, value: f32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_f64(data: &mut [u8], offset: usize, value: f64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn set_i8_array<const N: usize>(data: &mut [u8], offset: usize, values: &[i8; N]) {
    for (i, value) in values.iter().enumerate() {
        set_i8(data, offset + i, *value);
    }
}

pub(crate) fn set_u8_array<const N: usize>(data: &mut [u8], offset: usize, values: &[u8; N]) {
    data[offset..offset + N].copy_from_slice(values);
}

pub(crate) fn set_i16_array<const N: usize>(data: &mut [u8], offset: usize, values: &[i16; N]) {
    for (i, value) in values.iter().enumerate() {
        set_i16(data, offset + i * 2, *value);
    }
}

pub(crate) fn set_u16_array<const N: usize>(data: &mut [u8], offset: usize, values: &[u16; N]) {
    for (i, value) in values.iter().enumerate() {
        set_u16(data, offset + i * 2, *value);
    }
}

pub(crate) fn set_i32_array<const N: usize>(data: &mut [u8], offset: usize, values: &[i32; N]) {
    for (i, value) in values.iter().enumerate() {
        set_i32(data, offset + i * 4, *value);
    }
}

pub(crate) fn set_u32_array<const N: usize>(data: &mut [u8], offset: usize, values: &[u32; N]) {
    for (i, value) in values.iter().enumerate() {
        set_u32(data, offset + i * 4, *value);
    }
}

pub(crate) fn set_i64_array<const N: usize>(data: &mut [u8], offset: usize, values: &[i64; N]) {
    for (i, value) in values.iter().enumerate() {
        set_i64(data, offset + i * 8, *value);
    }
}

pub(crate) fn set_u64_array<const N: usize>(data: &mut [u8], offset: usize, values: &[u64; N]) {
    for (i, value) in values.iter().enumerate() {
        set_u64(data, offset + i * 8, *value);
    }
}

pub(crate) fn set_f32_array<const N: usize>(data: &mut [u8], offset: usize, values: &[f32; N]) {
    for (i, value) in values.iter().enumerate() {
        set_f32(data, offset + i * 4, *value);
    }
}

pub(crate) fn set_f64_array<const N: usize>(data: &mut [u8], offset: usize, values: &[f64; N]) {
    for (i, value) in values.iter().enumerate() {
        set_f64(data, offset + i * 8, *value);
    }
}
