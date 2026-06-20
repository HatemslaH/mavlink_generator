//! Heartbeat protocol example for the `rt_rc` dialect.

mod protocols_common;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{HeartbeatMonitor, HeartbeatPublisher, HeartbeatTemplates};

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = create_virtual_link(dialect);

    let gcs_publisher = HeartbeatPublisher::new(
        Arc::clone(&link.gcs),
        HeartbeatTemplates::gcs(dialect.version()),
        Duration::from_millis(500),
    );
    let drone_publisher = HeartbeatPublisher::new(
        Arc::clone(&link.drone),
        HeartbeatTemplates::autopilot_default(dialect.version()),
        Duration::from_millis(500),
    );
    let gcs_monitor = HeartbeatMonitor::new(
        Arc::clone(&link.gcs),
        Duration::from_secs(2),
        None,
        None,
    );

    gcs_monitor.start();
    gcs_publisher.start();
    drone_publisher.start();

    let mut exclude = HashSet::new();
    exclude.insert(GCS_SYSTEM_ID);
    let vehicle = gcs_monitor
        .wait_for_vehicle(Some(exclude), Duration::from_secs(5), None)
        .await
        .expect("vehicle should be discovered");
    println!("Vehicle discovered: {vehicle}");
    println!("Drone online: {}", gcs_monitor.is_online(vehicle));
    if let Some(state) = gcs_monitor.state_for(vehicle) {
        println!(
            "Drone heartbeat: type={:?} status={:?}",
            state.heartbeat.r#type, state.heartbeat.system_status
        );
    }

    drone_publisher.stop();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    println!("Drone online after stop: {}", gcs_monitor.is_online(vehicle));

    gcs_monitor.stop().await;
    gcs_publisher.stop();
    close_virtual_link(link).await.expect("close should succeed");
}
