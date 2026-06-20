//! Typed message subscription example for the `rt_rc` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::MavlinkNode;

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = create_virtual_link(dialect);
    let vehicle = MavlinkNode::new(DRONE_SYSTEM_ID, DRONE_COMPONENT_ID);

    let mut attitude_samples = Vec::new();
    let mut subscription = link.gcs.listen_message::<mavlink::Attitude, _>(
        |message, _frame| attitude_samples.push((*message).clone()),
        Some(vehicle.system_id),
        None,
    );

    link.drone
        .send(Box::new(mavlink::Attitude {
            time_boot_ms: 1000,
            roll: 0.1,
            pitch: -0.05,
            yaw: 1.57,
            rollspeed: 0.0,
            pitchspeed: 0.0,
            yawspeed: 0.0,
        }))
        .await
        .expect("send should succeed");

    tokio::time::sleep(Duration::from_millis(50)).await;
    subscription.cancel();

    println!(
        "Received {} ATTITUDE samples via listen_message",
        attitude_samples.len()
    );
    if let Some(sample) = attitude_samples.first() {
        println!(
            "  roll={} pitch={} yaw={}",
            sample.roll, sample.pitch, sample.yaw
        );
    }

    close_virtual_link(link).await.expect("close should succeed");
}
