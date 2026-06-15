/// MAVLink CRC-16/X.25 accumulator.
#[derive(Debug, Clone, Copy)]
pub struct CrcX25 {
    crc: u16,
}

impl CrcX25 {
    const X25_INIT_CRC: u16 = 0xFFFF;

    pub fn new() -> Self {
        Self {
            crc: Self::X25_INIT_CRC,
        }
    }

    pub fn crc(self) -> u16 {
        self.crc
    }

    pub fn accumulate(&mut self, byte: u8) {
        let mut tmp = byte ^ (self.crc as u8);
        tmp ^= tmp << 4;
        self.crc = (self.crc >> 8) ^ (u16::from(tmp) << 8) ^ (u16::from(tmp) << 3) ^ (u16::from(tmp) >> 4);
    }

    pub fn accumulate_str(&mut self, text: &str) {
        for byte in text.bytes() {
            self.accumulate(byte);
        }
    }
}

impl Default for CrcX25 {
    fn default() -> Self {
        Self::new()
    }
}
