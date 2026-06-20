//! Command protocol example for the `rt_rc` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{CommandProtocol, CommandServer};

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = create_virtual_link(dialect);

    let command_server = CommandServer::new(Arc::clone(&link.drone), None, None);
    let command_protocol = CommandProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(5),
    );

    let interval_ack = command_protocol
        .set_message_interval(mavlink::Attitude::MSG_ID, 100_000, None, None)
        .await
        .expect("set interval should succeed");
    println!("SET_MESSAGE_INTERVAL ack: {:?}", interval_ack.result);

    let request_ack = command_protocol
        .request_message(mavlink::Attitude::MSG_ID, 0.0, None, None)
        .await
        .expect("request message should succeed");
    println!("REQUEST_MESSAGE ack: {:?}", request_ack.result);

    let arm_ack = command_protocol
        .arm(false, None, None)
        .await
        .expect("arm should succeed");
    println!("ARM ack: {:?}", arm_ack.result);

    let disarm_ack = command_protocol
        .disarm(false, None, None)
        .await
        .expect("disarm should succeed");
    println!("DISARM ack: {:?}", disarm_ack.result);

    command_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}
