//! Mission protocol example for the `rt_rc` dialect.

mod protocols_common;

use std::sync::Arc;
use std::time::Duration;

use protocols_common::*;

use mavlink::protocols::{CommandProtocol, CommandServer, MissionItems, MissionProtocol, MissionServer};

#[tokio::main]
async fn main() {
    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = create_virtual_link(dialect);

    let mission_server = MissionServer::new(
        Arc::clone(&link.drone),
        None,
        mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
    );
    let _command_server = CommandServer::new(Arc::clone(&link.drone), None, None);
    let mission_protocol = MissionProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(3),
        Duration::from_secs(10),
    );

    let plan = vec![
        MissionItems::waypoint(
            0,
            47.397_742,
            8.545_594,
            50.0,
            DRONE_SYSTEM_ID,
            DRONE_COMPONENT_ID,
            mavlink::MavCmd::MAV_CMD_NAV_WAYPOINT,
            mavlink::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
        MissionItems::waypoint(
            1,
            47.398_000,
            8.546_000,
            50.0,
            DRONE_SYSTEM_ID,
            DRONE_COMPONENT_ID,
            mavlink::MavCmd::MAV_CMD_NAV_WAYPOINT,
            mavlink::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            1,
        ),
    ];

    let upload_result = mission_protocol
        .upload(
            plan,
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&|sent, total, item| {
                println!(
                    "Upload progress {}/{} seq={} cmd={:?}",
                    sent, total, item.seq, item.command
                );
            }),
            None,
        )
        .await
        .expect("upload should succeed");
    println!("Mission upload result: {upload_result:?}");
    println!("Vehicle stored {} items", mission_server.items().len());

    let downloaded = mission_protocol
        .download(
            mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&|received, total, item| {
                println!(
                    "Download progress {}/{} seq={}",
                    received, total, item.seq
                );
            }),
            None,
        )
        .await
        .expect("download should succeed");
    println!("Downloaded {} mission items", downloaded.len());

    let command_protocol = CommandProtocol::new(
        Arc::clone(&link.gcs),
        DRONE_SYSTEM_ID,
        DRONE_COMPONENT_ID,
        Duration::from_secs(5),
    );
    let set_current = mission_protocol
        .set_current_with_command(0, Some(&command_protocol), true, false, None)
        .await
        .expect("set current should succeed");
    println!(
        "Set current seq={} ack={:?}",
        set_current.sequence,
        set_current.command_ack.map(|ack| ack.result)
    );

    let clear_result = mission_protocol
        .clear(mavlink::MavMissionType::MAV_MISSION_TYPE_MISSION, None)
        .await
        .expect("clear should succeed");
    println!("Mission clear result: {clear_result:?}");

    mission_server.close().await;
    close_virtual_link(link).await.expect("close should succeed");
}
