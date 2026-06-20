//! Parameter protocol example for the `rt_rc` dialect.

mod protocols_common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{ParameterProtocol, ParameterServer};

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = create_virtual_link(dialect);

    let mut initial = HashMap::new();
    initial.insert(
        "SYSID_THISMAV".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    initial.insert(
        "SYSID_MYGCS".to_string(),
        (255.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );
    initial.insert(
        "COMPASS_ENABLE".to_string(),
        (1.0, mavlink::MavParamType::MAV_PARAM_TYPE_INT32),
    );

    let parameter_server = ParameterServer::from_typed(Arc::clone(&link.drone), initial);
    let parameter_protocol = ParameterProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_millis(500),
        Duration::from_secs(3),
    );

    let all_params = parameter_protocol
        .fetch_all(
            Some(&|entry, received, expected| {
                println!("  [{}/{}] {}={}", received, expected, entry.id, entry.value);
            }),
            None,
        )
        .await
        .expect("fetch all should succeed");
    println!(
        "Fetched {} parameters (cache size={})",
        all_params.len(),
        parameter_protocol.cache().len()
    );

    let single = parameter_protocol
        .read_by_name("SYSID_THISMAV", None)
        .await
        .expect("read should succeed");
    println!("Read SYSID_THISMAV={}", single.value);

    let updated = parameter_protocol
        .write_by_name("COMPASS_ENABLE", 0.0, None, None)
        .await
        .expect("write should succeed");
    println!("Wrote COMPASS_ENABLE={} ({:?})", updated.value, updated.param_type);

    parameter_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}
