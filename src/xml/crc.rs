pub struct CrcX25 {
    crc: u16,
}

impl CrcX25 {
    const INIT: u16 = 0xffff;

    pub fn new() -> Self {
        Self { crc: Self::INIT }
    }

    pub fn crc(&self) -> u16 {
        self.crc & 0xffff
    }

    pub fn accumulate(&mut self, byte: u8) {
        let byte = byte as u16;
        let mut tmp = byte ^ (self.crc & 0xff);
        tmp &= 0xff;
        tmp ^= (tmp << 4) & 0xff;

        self.crc = (self.crc >> 8) ^ ((tmp << 8) & 0xffff) ^ ((tmp << 3) & 0xffff) ^ (tmp >> 4);
    }

    pub fn accumulate_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.accumulate(byte);
        }
    }

    pub fn crc_extra(&self) -> u8 {
        let crc = self.crc();
        ((crc & 0xff) ^ (crc >> 8)) as u8
    }
}

impl Default for CrcX25 {
    fn default() -> Self {
        Self::new()
    }
}
