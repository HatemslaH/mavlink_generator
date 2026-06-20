use crate::mavlink_message::MavlinkMessage;

pub trait MavlinkDialect: Send + Sync {
    fn version(&self) -> u8;

    fn parse(&self, message_id: u32, data: &[u8]) -> Option<Box<dyn MavlinkMessage>>;

    /// Returns CRC extra for `message_id`, or -1 if unsupported.
    fn crc_extra(&self, message_id: u32) -> i32;
}
