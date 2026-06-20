//! MavlinkGcs / MavlinkVehicleClient facade example for `rt_rc`.

mod protocols_common;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{
    CommandServer, HeartbeatPublisher, HeartbeatTemplates, MavlinkGcs, MavlinkSession,
    ParameterServer, VirtualMavlinkBus,
};

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let bus = VirtualMavlinkBus::new();
    let gcs_link = bus.create_endpoint();
    let drone_link = bus.create_endpoint();

    let gcs = MavlinkGcs::connect(
        dialect.clone(),
        gcs_link,
        GCS_SYSTEM_ID,
        GCS_COMPONENT_ID,
        Duration::from_millis(500),
        Duration::from_secs(3),
    );

    let drone_session = Arc::new(MavlinkSession::new(
        dialect.clone(),
        drone_link,
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        mavlink::MavlinkVersion::V2,
    ));

    let drone_publisher = HeartbeatPublisher::new(
        Arc::clone(&drone_session),
        HeartbeatTemplates::autopilot_default(dialect.version()),
        Duration::from_millis(500),
    );

    let mut initial = HashMap::new();
    initial.insert(
        "SYSID_THISMAV".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    let parameter_server = ParameterServer::from_typed(Arc::clone(&drone_session), initial);
    let command_server = CommandServer::new(Arc::clone(&drone_session), None, None);

    gcs.start();
    drone_publisher.start();

    let mut exclude = HashSet::new();
    exclude.insert(GCS_SYSTEM_ID);
    let client = gcs
        .wait_for_vehicle(Some(exclude), Duration::from_secs(5))
        .await
        .expect("vehicle should connect");
    println!("Connected to vehicle {}", client.vehicle);

    let params = client
        .parameters
        .fetch_all(None, None)
        .await
        .expect("fetch all should succeed");
    println!("Vehicle has {} parameters", params.len());

    let ack = client
        .command
        .request_message(mavlink::Heartbeat::MSG_ID, 0.0, None, None)
        .await
        .expect("request message should succeed");
    println!("REQUEST_MESSAGE ack: {:?}", ack.result);

    parameter_server.close().await;
    command_server.close().await;
    drone_publisher.stop();
    drone_session.close().await.expect("close drone session");
    gcs.close().await.expect("close gcs");
    bus.close_all().await;
}
