use crate::crc::CrcX25;
use crate::mavlink_dialect::MavlinkDialect;
use crate::mavlink_frame::MavlinkFrame;
use crate::mavlink_version::MavlinkVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Init,
    WaitPayloadLength,
    WaitIncompatibilityFlags,
    WaitCompatibilityFlags,
    WaitPacketSequence,
    WaitSystemId,
    WaitComponentId,
    WaitMessageIdLow,
    WaitMessageIdMiddle,
    WaitMessageIdHigh,
    WaitPayloadEnd,
    WaitCrcLowByte,
    WaitCrcHighByte,
    WaitSignatureTrailer,
}

pub struct MavlinkParser {
    dialect: Box<dyn MavlinkDialect>,
    on_signed_packet_dropped: Option<Box<dyn FnMut(u32) + Send>>,
    frames: Vec<MavlinkFrame>,
    state: ParserState,
    version: MavlinkVersion,
    payload_length: i32,
    incompatibility_flags: i32,
    compatibility_flags: i32,
    sequence: i32,
    system_id: i32,
    component_id: i32,
    message_id_low: i32,
    message_id_middle: i32,
    message_id_high: i32,
    message_id: i32,
    payload: [u8; 255],
    payload_cursor: i32,
    crc_low_byte: i32,
    crc_high_byte: i32,
    signature_bytes_remaining: i32,
}

impl MavlinkParser {
    const MAVLINK_MAXIMUM_PAYLOAD_SIZE: usize = 255;
    const MAVLINK_IFLAG_SIGNED: u8 = 0x01;
    const MAVLINK_SIGNATURE_LENGTH: i32 = 13;

    pub fn new(dialect: Box<dyn MavlinkDialect>) -> Self {
        let mut parser = Self {
            dialect,
            on_signed_packet_dropped: None,
            frames: Vec::new(),
            state: ParserState::Init,
            version: MavlinkVersion::V1,
            payload_length: -1,
            incompatibility_flags: -1,
            compatibility_flags: -1,
            sequence: -1,
            system_id: -1,
            component_id: -1,
            message_id_low: -1,
            message_id_middle: -1,
            message_id_high: -1,
            message_id: -1,
            payload: [0; Self::MAVLINK_MAXIMUM_PAYLOAD_SIZE],
            payload_cursor: -1,
            crc_low_byte: -1,
            crc_high_byte: -1,
            signature_bytes_remaining: 0,
        };
        parser.reset_context();
        parser
    }

    pub fn frames(&self) -> &[MavlinkFrame] {
        &self.frames
    }

    pub fn set_on_signed_packet_dropped(&mut self, callback: Box<dyn FnMut(u32) + Send>) {
        self.on_signed_packet_dropped = Some(callback);
    }

    pub fn parse(&mut self, data: &[u8]) {
        for &byte in data {
            self.parse_byte(byte);
        }
    }

    fn reset_context(&mut self) {
        self.version = MavlinkVersion::V1;
        self.payload_length = -1;
        self.incompatibility_flags = -1;
        self.compatibility_flags = -1;
        self.sequence = -1;
        self.system_id = -1;
        self.component_id = -1;
        self.message_id_low = -1;
        self.message_id_middle = -1;
        self.message_id_high = -1;
        self.message_id = -1;
        self.payload = [0; Self::MAVLINK_MAXIMUM_PAYLOAD_SIZE];
        self.payload_cursor = -1;
        self.crc_low_byte = -1;
        self.crc_high_byte = -1;
        self.signature_bytes_remaining = 0;
    }

    fn check_crc(&self) -> bool {
        let mut crc = CrcX25::new();
        let header: Vec<u8> = if self.version == MavlinkVersion::V1 {
            vec![
                self.payload_length as u8,
                self.sequence as u8,
                self.system_id as u8,
                self.component_id as u8,
                self.message_id as u8,
            ]
        } else {
            vec![
                self.payload_length as u8,
                self.incompatibility_flags as u8,
                self.compatibility_flags as u8,
                self.sequence as u8,
                self.system_id as u8,
                self.component_id as u8,
                self.message_id_low as u8,
                self.message_id_middle as u8,
                self.message_id_high as u8,
            ]
        };

        for value in header {
            crc.accumulate(value);
        }
        for i in 0..self.payload_length as usize {
            crc.accumulate(self.payload[i]);
        }

        let crc_ext = self.dialect.crc_extra(self.message_id as u32);
        if crc_ext == -1 {
            return false;
        }
        crc.accumulate(crc_ext as u8);

        let expected = ((self.crc_high_byte as u16) << 8) | (self.crc_low_byte as u16);
        crc.crc() == expected
    }

    fn parse_byte(&mut self, byte: u8) {
        match self.state {
            ParserState::Init => {
                if byte == MavlinkFrame::MAVLINK_STX_V1 {
                    self.version = MavlinkVersion::V1;
                    self.state = ParserState::WaitPayloadLength;
                } else if byte == MavlinkFrame::MAVLINK_STX_V2 {
                    self.version = MavlinkVersion::V2;
                    self.state = ParserState::WaitPayloadLength;
                }
            }
            ParserState::WaitPayloadLength => {
                self.payload_length = i32::from(byte);
                self.state = if self.version == MavlinkVersion::V1 {
                    ParserState::WaitPacketSequence
                } else {
                    ParserState::WaitIncompatibilityFlags
                };
            }
            ParserState::WaitIncompatibilityFlags => {
                self.incompatibility_flags = i32::from(byte);
                self.state = ParserState::WaitCompatibilityFlags;
            }
            ParserState::WaitCompatibilityFlags => {
                self.compatibility_flags = i32::from(byte);
                self.state = ParserState::WaitPacketSequence;
            }
            ParserState::WaitPacketSequence => {
                self.sequence = i32::from(byte);
                self.state = ParserState::WaitSystemId;
            }
            ParserState::WaitSystemId => {
                self.system_id = i32::from(byte);
                self.state = ParserState::WaitComponentId;
            }
            ParserState::WaitComponentId => {
                self.component_id = i32::from(byte);
                self.state = if self.version == MavlinkVersion::V1 {
                    ParserState::WaitMessageIdHigh
                } else {
                    ParserState::WaitMessageIdLow
                };
            }
            ParserState::WaitMessageIdLow => {
                self.message_id_low = i32::from(byte);
                self.state = ParserState::WaitMessageIdMiddle;
            }
            ParserState::WaitMessageIdMiddle => {
                self.message_id_middle = i32::from(byte);
                self.state = ParserState::WaitMessageIdHigh;
            }
            ParserState::WaitMessageIdHigh => {
                if self.version == MavlinkVersion::V1 {
                    self.message_id = i32::from(byte);
                } else {
                    self.message_id_high = i32::from(byte);
                    self.message_id = (self.message_id_high << 16)
                        ^ (self.message_id_middle << 8)
                        ^ self.message_id_low;
                }
                self.state = if self.payload_length == 0 {
                    ParserState::WaitCrcLowByte
                } else {
                    self.payload_cursor = 0;
                    ParserState::WaitPayloadEnd
                };
            }
            ParserState::WaitPayloadEnd => {
                if self.payload_cursor < self.payload_length {
                    self.payload[self.payload_cursor as usize] = byte;
                    self.payload_cursor += 1;
                }
                if self.payload_cursor == self.payload_length {
                    self.state = ParserState::WaitCrcLowByte;
                }
            }
            ParserState::WaitCrcLowByte => {
                self.crc_low_byte = i32::from(byte);
                self.state = ParserState::WaitCrcHighByte;
            }
            ParserState::WaitCrcHighByte => {
                self.crc_high_byte = i32::from(byte);
                if self.version == MavlinkVersion::V2
                    && (self.incompatibility_flags as u8 & Self::MAVLINK_IFLAG_SIGNED) != 0
                {
                    if let Some(callback) = self.on_signed_packet_dropped.as_mut() {
                        callback(self.message_id as u32);
                    }
                    self.signature_bytes_remaining = Self::MAVLINK_SIGNATURE_LENGTH;
                    self.state = ParserState::WaitSignatureTrailer;
                    return;
                }

                self.add_mavlink_frame();
                self.reset_context();
                self.state = ParserState::Init;
            }
            ParserState::WaitSignatureTrailer => {
                self.signature_bytes_remaining -= 1;
                if self.signature_bytes_remaining == 0 {
                    self.reset_context();
                    self.state = ParserState::Init;
                }
            }
        }
    }

    fn add_mavlink_frame(&mut self) {
        if !self.check_crc() {
            return;
        }

        let payload = &self.payload[..self.payload_length as usize];
        let Some(message) = self.dialect.parse(self.message_id as u32, payload) else {
            return;
        };

        self.frames.push(MavlinkFrame {
            version: self.version,
            sequence: self.sequence as u8,
            system_id: self.system_id as u8,
            component_id: self.component_id as u8,
            message,
        });
    }
}
