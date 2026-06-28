//! Interactive MAVLink GCS over serial (SITL / USB COM).

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mavlink::{
    protocols::{MavlinkCancellationToken, MavlinkGcs, SessionWaitError},
    Attitude, MavMissionType, MavParamType, MavlinkDialectRtRc,
};
use mavlink_sitl_gcs::{
    build_sample_mission, describe_mission_item, parse_baud_rate, pick_serial_port, GcsContext,
    SerialMavlinkLink, GCS_COMPONENT_ID, GCS_SYSTEM_ID,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let baud_rate = parse_baud_rate(&args, 57_600)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let port_name = pick_serial_port()?;

    println!();
    println!("Opening {port_name} @ {baud_rate} baud...");

    let dialect: Arc<dyn mavlink::MavlinkDialect + Send + Sync> = Arc::new(MavlinkDialectRtRc);
    let link = SerialMavlinkLink::open(&port_name, baud_rate)?;
    let gcs = MavlinkGcs::connect(
        dialect,
        link,
        GCS_SYSTEM_ID,
        GCS_COMPONENT_ID,
        Duration::from_secs(1),
        Duration::from_secs(3),
    );

    gcs.start();
    println!("Publishing GCS heartbeats, waiting for vehicle...");

    let client = match gcs
        .wait_for_vehicle(
            Some(HashSet::from([GCS_SYSTEM_ID])),
            Duration::from_secs(60),
        )
        .await
    {
        Ok(client) => client,
        Err(SessionWaitError::Timeout(_)) => {
            return Err(format!(
                "No vehicle heartbeat within 60 s. Check port, baud (current: {baud_rate}; try --baud 115200), and SITL."
            )
            .into());
        }
        Err(error) => return Err(error.into()),
    };

    let vehicle = client.vehicle;
    if let Some(state) = gcs.heartbeat_monitor.state_for(vehicle) {
        println!("Vehicle online: {vehicle}");
        println!(
            "  type={:?} autopilot={:?} status={:?}",
            state.heartbeat.r#type, state.heartbeat.autopilot, state.heartbeat.system_status
        );
    } else {
        println!("Vehicle online: {vehicle}");
    }

    let mut ctx = GcsContext::new(gcs, vehicle, client);

    println!();
    println!("=== Phase 2: parameter sync ===");
    fetch_all_parameters(&mut ctx).await?;

    println!();
    println!("=== Interactive CLI ===");
    run_cli(&mut ctx).await?;

    println!("Shutting down...");
    if let Some(cancel) = ctx.operation_cancel {
        cancel.cancel();
    }
    ctx.gcs.close().await?;
    Ok(())
}

async fn fetch_all_parameters(ctx: &mut GcsContext) -> Result<(), SessionWaitError> {
    let cancel = MavlinkCancellationToken::new();
    ctx.operation_cancel = Some(cancel.clone());

    println!("[parameters] waiting for PARAM_VALUE stream...");
    let on_progress = |entry: &mavlink::protocols::ParamEntry, received: usize, expected: u16| {
        if received == 1 {
            println!("[parameters] expecting {expected} parameters");
        }
        println!(
            "[parameters] {received}/{expected} {}={} ({:?})",
            entry.id, entry.value, entry.param_type
        );
    };

    let entries = ctx
        .parameters()
        .fetch_all(Some(&on_progress), Some(&cancel))
        .await?;
    println!(
        "[parameters] complete ({} total, cache={})",
        entries.len(),
        ctx.parameters().cache().len()
    );
    Ok(())
}

async fn run_cli(ctx: &mut GcsContext) -> Result<(), SessionWaitError> {
    print_help();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    while let Some(line) = lines.next() {
        let line = line.map_err(|error| SessionWaitError::Closed)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts[0].to_ascii_lowercase();

        let result = match command.as_str() {
            "h" | "help" => {
                print_help();
                Ok(())
            }
            "q" | "quit" | "exit" => return Ok(()),
            "hb" => {
                print_heartbeat_status(ctx);
                Ok(())
            }
            "cancel" => {
                cancel_operation(ctx);
                Ok(())
            }
            "p" | "params" => fetch_all_parameters(ctx).await,
            "pr" => read_parameter(ctx, &parts).await,
            "pw" => write_parameter(ctx, &parts).await,
            "mu" => upload_mission(ctx).await,
            "md" => download_mission(ctx).await,
            "mc" => clear_mission(ctx).await,
            "ms" => set_mission_current(ctx, &parts).await,
            "rm" => request_message(ctx, &parts).await,
            "si" => set_message_interval(ctx, &parts).await,
            "att" => stream_attitude(ctx, &parts).await,
            "arm" => arm(ctx, &parts).await,
            "disarm" => disarm(ctx, &parts).await,
            "rtl" => return_to_launch(ctx).await,
            other => {
                println!("Unknown command: {other} (type help)");
                Ok(())
            }
        };

        match result {
            Ok(()) => {}
            Err(SessionWaitError::Cancelled(_)) => println!("Operation cancelled."),
            Err(error) => println!("Error: {error}"),
        }

        println!();
        print!("gcs> ");
        io::stdout().flush().ok();
    }

    Ok(())
}

fn print_help() {
    println!("Commands:");
    println!("  help              Show this help");
    println!("  hb                Heartbeat / link status");
    println!("  cancel            Cancel in-flight params/mission operation");
    println!("  params            Request full parameter list (with progress)");
    println!("  pr <name>         Read one parameter by name");
    println!("  pw <name> <value> Write parameter (type from cache or REAL32)");
    println!("  mu                Upload hardcoded sample mission");
    println!("  md                Download mission from vehicle");
    println!("  mc                Clear onboard mission");
    println!("  ms <seq>          Set active mission item (mission + command)");
    println!("  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)");
    println!("  si <msgId> <us>   Set message interval (microseconds)");
    println!("  att [seconds]     Stream ATTITUDE via listen_message (default 5 s)");
    println!("  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)");
    println!("  disarm [force]    Disarm motors");
    println!("  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH");
    println!("  quit              Exit");
    print!("gcs> ");
    let _ = io::stdout().flush();
}

fn cancel_operation(ctx: &GcsContext) {
    match &ctx.operation_cancel {
        Some(token) if !token.is_cancelled() => {
            token.cancel();
            println!("[cancel] signalled");
        }
        _ => println!("[cancel] no active cancellable operation"),
    }
}

fn print_heartbeat_status(ctx: &GcsContext) {
    let node = ctx.vehicle;
    let online = ctx.heartbeat_monitor().is_online(node);
    println!("[heartbeat] vehicle {node} online={online}");
    if let Some(state) = ctx.heartbeat_monitor().state_for(node) {
        println!(
            "  last={}ms ago type={:?} status={:?}",
            state.age().as_millis(),
            state.heartbeat.r#type,
            state.heartbeat.system_status
        );
    } else {
        println!("  no heartbeat received yet");
    }
}

async fn read_parameter(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    if parts.len() < 2 {
        println!("Usage: pr <name>");
        return Ok(());
    }

    let name = parts[1];
    println!("[parameters] reading {name}...");
    let entry = ctx.parameters().read_by_name(name, None).await?;
    println!(
        "[parameters] {name}={} ({:?}, index {}/{})",
        entry.value, entry.param_type, entry.index, entry.count
    );
    Ok(())
}

async fn write_parameter(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    if parts.len() < 3 {
        println!("Usage: pw <name> <value>");
        return Ok(());
    }

    let name = parts[1];
    let raw_value = parts[2];
    let param_type = ctx
        .parameters()
        .type_for_name(name)
        .unwrap_or(MavParamType::MAV_PARAM_TYPE_REAL32);
    let value = match parse_param_value(raw_value, param_type) {
        Ok(value) => value,
        Err(error) => {
            println!("Error: {error}");
            return Ok(());
        }
    };

    println!("[parameters] writing {name}={value} ({param_type:?})...");
    let entry = ctx
        .parameters()
        .write_by_name(name, value, None, None)
        .await?;
    println!(
        "[parameters] ack {name}={} ({:?})",
        entry.value, entry.param_type
    );
    Ok(())
}

fn parse_param_value(raw: &str, param_type: MavParamType) -> Result<f64, String> {
    match param_type {
        MavParamType::MAV_PARAM_TYPE_INT8
        | MavParamType::MAV_PARAM_TYPE_INT16
        | MavParamType::MAV_PARAM_TYPE_INT32
        | MavParamType::MAV_PARAM_TYPE_INT64 => raw
            .parse::<i64>()
            .map(|value| value as f64)
            .map_err(|_| format!("invalid signed integer parameter value: {raw}")),
        MavParamType::MAV_PARAM_TYPE_UINT8
        | MavParamType::MAV_PARAM_TYPE_UINT16
        | MavParamType::MAV_PARAM_TYPE_UINT32
        | MavParamType::MAV_PARAM_TYPE_UINT64 => raw
            .parse::<u64>()
            .map(|value| value as f64)
            .map_err(|_| format!("invalid unsigned integer parameter value: {raw}")),
        _ => raw
            .parse::<f64>()
            .map_err(|_| format!("invalid float parameter value: {raw}")),
    }
}

async fn upload_mission(ctx: &mut GcsContext) -> Result<(), SessionWaitError> {
    let plan = build_sample_mission(ctx.target_system(), ctx.target_component());
    let cancel = MavlinkCancellationToken::new();
    ctx.operation_cancel = Some(cancel.clone());

    println!("[mission] uploading {} hardcoded items...", plan.len());
    let on_progress = |sent: usize, total: usize, item: &mavlink::MissionItemInt| {
        println!(
            "[mission upload] {sent}/{total} seq={} {}",
            item.seq,
            describe_mission_item(item)
        );
    };

    let result = ctx
        .mission()
        .upload(
            plan,
            MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&on_progress),
            Some(&cancel),
        )
        .await?;
    println!("[mission] upload finished: {result:?}");
    Ok(())
}

async fn download_mission(ctx: &mut GcsContext) -> Result<(), SessionWaitError> {
    let cancel = MavlinkCancellationToken::new();
    ctx.operation_cancel = Some(cancel.clone());

    let on_progress = |received: usize, total: u16, item: &mavlink::MissionItemInt| {
        println!(
            "[mission download] {received}/{total} {}",
            describe_mission_item(item)
        );
    };

    let items = ctx
        .mission()
        .download(
            MavMissionType::MAV_MISSION_TYPE_MISSION,
            Some(&on_progress),
            Some(&cancel),
        )
        .await?;
    println!("[mission] on vehicle:");
    for item in &items {
        println!("  {}", describe_mission_item(item));
    }
    Ok(())
}

async fn clear_mission(ctx: &GcsContext) -> Result<(), SessionWaitError> {
    println!("[mission] sending MISSION_CLEAR_ALL...");
    let result = ctx
        .mission()
        .clear(MavMissionType::MAV_MISSION_TYPE_MISSION, None)
        .await?;
    println!("[mission] clear result: {result:?}");
    Ok(())
}

async fn set_mission_current(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    if parts.len() < 2 {
        println!("Usage: ms <seq>");
        return Ok(());
    }

    let seq: u16 = parts[1].parse().map_err(|_| SessionWaitError::Closed)?;
    println!("[mission] set current seq={seq} (mission + command)...");
    let result = ctx
        .mission()
        .set_current_with_command(seq, Some(ctx.command()), true, false, None)
        .await?;
    let ack = result
        .command_ack
        .as_ref()
        .map(|ack| format!("{:?}", ack.result))
        .unwrap_or_else(|| "n/a".to_string());
    println!("[mission] seq={} command ack={ack}", result.sequence);
    Ok(())
}

async fn request_message(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    if parts.len() < 2 {
        println!(
            "Usage: rm <msgId>  (e.g. rm {} for ATTITUDE)",
            Attitude::MSG_ID
        );
        return Ok(());
    }

    let msg_id: u32 = parts[1].parse().map_err(|_| SessionWaitError::Closed)?;
    println!("[command] REQUEST_MESSAGE id={msg_id}");
    let ack = ctx
        .command()
        .request_message(msg_id, 0.0, None, None)
        .await?;
    println!("[command] ack: {:?}", ack.result);

    if msg_id == Attitude::MSG_ID {
        println!("[telemetry] waiting for ATTITUDE...");
        let attitude = ctx
            .session()
            .wait_for_message_type::<Attitude>(
                Some(ctx.target_system()),
                None,
                Duration::from_secs(5),
                None,
            )
            .await?;
        println!(
            "[telemetry] roll={} pitch={} yaw={}",
            attitude.roll, attitude.pitch, attitude.yaw
        );
    }
    Ok(())
}

async fn set_message_interval(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    if parts.len() < 3 {
        println!("Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)");
        return Ok(());
    }

    let msg_id: u32 = parts[1].parse().map_err(|_| SessionWaitError::Closed)?;
    let interval_us: u32 = parts[2].parse().map_err(|_| SessionWaitError::Closed)?;
    println!("[command] SET_MESSAGE_INTERVAL id={msg_id} interval={interval_us} us");
    let ack = if interval_us == 0 {
        ctx.command()
            .stop_message_interval(msg_id, None, None)
            .await?
    } else {
        ctx.command()
            .set_message_interval(msg_id, interval_us, None, None)
            .await?
    };
    println!("[command] ack: {:?}", ack.result);
    Ok(())
}

async fn stream_attitude(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    let seconds: u64 = if parts.len() >= 2 {
        parts[1].parse().map_err(|_| SessionWaitError::Closed)?
    } else {
        5
    };
    println!("[telemetry] streaming ATTITUDE for {seconds}s (subscribe + interval)...");

    ctx.command()
        .set_message_interval(Attitude::MSG_ID, 100_000, None, None)
        .await?;

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);
    let target_system = ctx.target_system();
    let mut subscription = ctx.session().listen_message::<Attitude, _>(
        move |attitude, _frame| {
            let n = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!(
                "[attitude] #{n} roll={:.3} pitch={:.3} yaw={:.3}",
                attitude.roll, attitude.pitch, attitude.yaw
            );
        },
        Some(target_system),
        None,
    );

    tokio::time::sleep(Duration::from_secs(seconds)).await;
    subscription.cancel();
    ctx.command()
        .stop_message_interval(Attitude::MSG_ID, None, None)
        .await?;
    println!(
        "[telemetry] received {} ATTITUDE messages",
        count.load(Ordering::SeqCst)
    );
    Ok(())
}

async fn arm(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    let force = parts.len() >= 2 && parts[1].eq_ignore_ascii_case("force");
    println!("[command] ARM{}...", if force { " (force)" } else { "" });
    let ack = ctx.command().arm(force, None, None).await?;
    println!("[command] ack: {:?}", ack.result);
    Ok(())
}

async fn disarm(ctx: &GcsContext, parts: &[&str]) -> Result<(), SessionWaitError> {
    let force = parts.len() >= 2 && parts[1].eq_ignore_ascii_case("force");
    println!("[command] DISARM{}...", if force { " (force)" } else { "" });
    let ack = ctx.command().disarm(force, None, None).await?;
    println!("[command] ack: {:?}", ack.result);
    Ok(())
}

async fn return_to_launch(ctx: &GcsContext) -> Result<(), SessionWaitError> {
    println!("[command] RETURN_TO_LAUNCH...");
    let ack = ctx.command().return_to_launch(None, None).await?;
    println!("[command] ack: {:?}", ack.result);
    Ok(())
}
