use crate::crc::CrcX25;
use crate::mavlink_message::MavlinkMessage;
use crate::mavlink_version::MavlinkVersion;

pub struct MavlinkFrame {
    pub version: MavlinkVersion,
    pub sequence: u8,
    pub system_id: u8,
    pub component_id: u8,
    pub message: Box<dyn MavlinkMessage>,
}

impl MavlinkFrame {
    pub const MAVLINK_STX_V1: u8 = 0xFE;
    pub const MAVLINK_STX_V2: u8 = 0xFD;

    pub fn v1(
        sequence: u8,
        system_id: u8,
        component_id: u8,
        message: Box<dyn MavlinkMessage>,
    ) -> Self {
        Self {
            version: MavlinkVersion::V1,
            sequence,
            system_id,
            component_id,
            message,
        }
    }

    pub fn v2(
        sequence: u8,
        system_id: u8,
        component_id: u8,
        message: Box<dyn MavlinkMessage>,
    ) -> Self {
        Self {
            version: MavlinkVersion::V2,
            sequence,
            system_id,
            component_id,
            message,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        match self.version {
            MavlinkVersion::V1 => self.serialize_v1(),
            MavlinkVersion::V2 => self.serialize_v2(),
        }
    }

    fn serialize_v1(&self) -> Vec<u8> {
        let payload = self.message.serialize();
        let payload_length = payload.len();
        let mut frame = vec![0u8; 8 + payload_length];
        frame[0] = Self::MAVLINK_STX_V1;
        frame[1] = payload_length as u8;
        frame[2] = self.sequence;
        frame[3] = self.system_id;
        frame[4] = self.component_id;
        frame[5] = self.message.mavlink_message_id() as u8;

        let mut crc = CrcX25::new();
        crc.accumulate(frame[1]);
        crc.accumulate(frame[2]);
        crc.accumulate(frame[3]);
        crc.accumulate(frame[4]);
        crc.accumulate(frame[5]);

        for (i, byte) in payload.iter().enumerate() {
            frame[6 + i] = *byte;
            crc.accumulate(*byte);
        }
        crc.accumulate(self.message.mavlink_crc_extra());

        let checksum = crc.crc();
        let frame_len = frame.len();
        frame[frame_len - 2] = (checksum & 0xFF) as u8;
        frame[frame_len - 1] = (checksum >> 8) as u8;
        frame
    }

    fn serialize_v2(&self) -> Vec<u8> {
        let incompatibility_flags = 0u8;
        let compatibility_flags = 0u8;
        let payload = Self::trim_trailing_zeros(&self.message.serialize());
        let payload_length = payload.len();
        let message_id = self.message.mavlink_message_id();
        let message_id_bytes = [
            (message_id & 0xFF) as u8,
            ((message_id >> 8) & 0xFF) as u8,
            ((message_id >> 16) & 0xFF) as u8,
        ];

        let mut frame = vec![0u8; 12 + payload_length];
        frame[0] = Self::MAVLINK_STX_V2;
        frame[1] = payload_length as u8;
        frame[2] = incompatibility_flags;
        frame[3] = compatibility_flags;
        frame[4] = self.sequence;
        frame[5] = self.system_id;
        frame[6] = self.component_id;
        frame[7] = message_id_bytes[0];
        frame[8] = message_id_bytes[1];
        frame[9] = message_id_bytes[2];

        let mut crc = CrcX25::new();
        crc.accumulate(frame[1]);
        crc.accumulate(frame[2]);
        crc.accumulate(frame[3]);
        crc.accumulate(frame[4]);
        crc.accumulate(frame[5]);
        crc.accumulate(frame[6]);
        for byte in message_id_bytes {
            crc.accumulate(byte);
        }

        for (i, byte) in payload.iter().enumerate() {
            frame[10 + i] = *byte;
            crc.accumulate(*byte);
        }
        crc.accumulate(self.message.mavlink_crc_extra());

        let checksum = crc.crc();
        let frame_len = frame.len();
        frame[frame_len - 2] = (checksum & 0xFF) as u8;
        frame[frame_len - 1] = (checksum >> 8) as u8;
        frame
    }

    fn trim_trailing_zeros(payload: &[u8]) -> Vec<u8> {
        let mut trimmed_length = payload.len();
        while trimmed_length > 0 && payload[trimmed_length - 1] == 0 {
            trimmed_length -= 1;
        }
        payload[..trimmed_length].to_vec()
    }
}
