#!/usr/bin/env python3
"""Interactive MAVLink GCS over serial (SITL) using generated bindings and protocol classes."""

from __future__ import annotations

import asyncio
import sys
from datetime import timedelta
from numbers import Real

import bindings  # noqa: F401

from dialects.rt_rc import Attitude, MavlinkDialectRt_rc  # noqa: E402
from gcs_context import GcsContext, gcs_component_id, gcs_system_id  # noqa: E402
from mavlink import MavParamType  # noqa: E402
from mavlink_protocols import (  # noqa: E402
    MavlinkCancelledException,
    MavlinkCancellationToken,
    MavlinkGcs,
    MavlinkTimeoutException,
)
from port_picker import parse_baud_rate, pick_serial_port  # noqa: E402
from sample_mission import build_sample_mission, describe_mission_item  # noqa: E402
from serial_link import SerialMavlinkLink  # noqa: E402


async def main() -> None:
    baud_rate = parse_baud_rate(sys.argv[1:])
    port_name = pick_serial_port()

    print()
    print(f"Opening {port_name} @ {baud_rate} baud...")

    dialect = MavlinkDialectRt_rc()
    link = SerialMavlinkLink.open(port_name, baud_rate=baud_rate)
    gcs = MavlinkGcs.connect(
        dialect=dialect,
        link=link,
        system_id=gcs_system_id,
        component_id=gcs_component_id,
    )

    gcs.start()
    print("Publishing GCS heartbeats, waiting for vehicle...")

    try:
        client = await gcs.wait_for_vehicle(
            exclude_system_ids={gcs_system_id},
            timeout=timedelta(seconds=60),
        )
    except MavlinkTimeoutException:
        raise RuntimeError(
            f"No vehicle heartbeat within 60 s. Check port, baud "
            f"(current: {baud_rate}; try --baud 115200), and SITL."
        ) from None

    vehicle = client.vehicle
    vehicle_state = gcs.heartbeat_monitor.state_for(vehicle)
    print(f"Vehicle online: {vehicle}")
    if vehicle_state is not None:
        hb = vehicle_state.heartbeat
        print(
            f"  type={hb.type.name} "
            f"autopilot={hb.autopilot.name} "
            f"status={hb.system_status.name}"
        )

    ctx = GcsContext(gcs=gcs, vehicle=vehicle, client=client)

    print()
    print("=== Phase 2: parameter sync ===")
    await fetch_all_parameters(ctx)

    print()
    print("=== Interactive CLI ===")
    await run_cli(ctx)

    print("Shutting down...")
    if ctx.operation_cancel is not None:
        ctx.operation_cancel.cancel()
    await gcs.close()


async def fetch_all_parameters(ctx: GcsContext) -> None:
    cancel = MavlinkCancellationToken()
    ctx.operation_cancel = cancel

    print("[parameters] waiting for PARAM_VALUE stream...")
    entries = await ctx.parameters.fetch_all(
        cancel=cancel,
        on_progress=lambda entry, received, expected: _on_param_progress(
            entry, received, expected
        ),
    )
    print(
        f"[parameters] complete ({len(entries)} total, "
        f"cache={len(ctx.parameters.cache)})"
    )


def _on_param_progress(entry, received: int, expected: int) -> None:
    if received == 1:
        print(f"[parameters] expecting {expected} parameters")
    print(
        f"[parameters] {received}/{expected} "
        f"{entry.id}={entry.value} ({entry.type.name})"
    )


async def run_cli(ctx: GcsContext) -> None:
    print_help()
    loop = asyncio.get_running_loop()

    while True:
        print("gcs> ", end="", flush=True)
        line = await loop.run_in_executor(None, sys.stdin.readline)
        if not line:
            break

        trimmed = line.strip()
        if not trimmed:
            continue

        parts = trimmed.split()
        command = parts[0].lower()

        try:
            if command in ("h", "help"):
                print_help()
            elif command in ("q", "quit", "exit"):
                return
            elif command == "hb":
                print_heartbeat_status(ctx)
            elif command == "cancel":
                cancel_operation(ctx)
            elif command in ("p", "params"):
                await fetch_all_parameters(ctx)
            elif command == "pr":
                await read_parameter(ctx, parts)
            elif command == "pw":
                await write_parameter(ctx, parts)
            elif command == "mu":
                await upload_mission(ctx)
            elif command == "md":
                await download_mission(ctx)
            elif command == "mc":
                await clear_mission(ctx)
            elif command == "ms":
                await set_mission_current(ctx, parts)
            elif command == "rm":
                await request_message(ctx, parts)
            elif command == "si":
                await set_message_interval(ctx, parts)
            elif command == "att":
                await stream_attitude(ctx, parts)
            elif command == "arm":
                await arm(ctx, parts)
            elif command == "disarm":
                await disarm(ctx, parts)
            elif command == "rtl":
                await return_to_launch(ctx)
            else:
                print(f"Unknown command: {command} (type help)")
        except MavlinkCancelledException:
            print("Operation cancelled.")
        except Exception as error:
            print(f"Error: {error}")

        print()


def print_help() -> None:
    print("Commands:")
    print("  help              Show this help")
    print("  hb                Heartbeat / link status")
    print("  cancel            Cancel in-flight params/mission operation")
    print("  params            Request full parameter list (with progress)")
    print("  pr <name>         Read one parameter by name")
    print("  pw <name> <value> Write parameter (type from cache or REAL32)")
    print("  mu                Upload hardcoded sample mission")
    print("  md                Download mission from vehicle")
    print("  mc                Clear onboard mission")
    print("  ms <seq>          Set active mission item (mission + command)")
    print("  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)")
    print("  si <msgId> <us>   Set message interval (microseconds)")
    print("  att [seconds]     Stream ATTITUDE via onMessage (default 5 s)")
    print("  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)")
    print("  disarm [force]    Disarm motors")
    print("  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH")
    print("  quit              Exit")


def cancel_operation(ctx: GcsContext) -> None:
    token = ctx.operation_cancel
    if token is None or token.is_cancelled:
        print("[cancel] no active cancellable operation")
        return
    token.cancel()
    print("[cancel] signalled")


def print_heartbeat_status(ctx: GcsContext) -> None:
    node = ctx.vehicle
    online = ctx.heartbeat_monitor.is_online(node)
    state = ctx.heartbeat_monitor.state_for(node)

    print(f"[heartbeat] vehicle {node} online={online}")
    if state is not None:
        print(
            f"  last={int(state.age.total_seconds() * 1000)}ms ago "
            f"type={state.heartbeat.type.name} "
            f"status={state.heartbeat.system_status.name}"
        )
    else:
        print("  no heartbeat received yet")


async def read_parameter(ctx: GcsContext, parts: list[str]) -> None:
    if len(parts) < 2:
        print("Usage: pr <name>")
        return

    name = parts[1]
    print(f"[parameters] reading {name}...")
    entry = await ctx.parameters.read_by_name(name)
    print(
        f"[parameters] {name}={entry.value} ({entry.type.name}, "
        f"index {entry.index}/{entry.count})"
    )


async def write_parameter(ctx: GcsContext, parts: list[str]) -> None:
    if len(parts) < 3:
        print("Usage: pw <name> <value>")
        return

    name = parts[1]
    raw_value = parts[2]
    cached_type = ctx.parameters.type_for_name(name)
    param_type = cached_type or MavParamType.MAV_PARAM_TYPE_REAL32
    value = parse_param_value(raw_value, param_type)

    print(f"[parameters] writing {name}={value} ({param_type.name})...")
    entry = await ctx.parameters.write_by_name(name, value)
    print(f"[parameters] ack {name}={entry.value} ({entry.type.name})")


def parse_param_value(raw: str, param_type: MavParamType) -> Real:
    if param_type in (
        MavParamType.MAV_PARAM_TYPE_INT8,
        MavParamType.MAV_PARAM_TYPE_INT16,
        MavParamType.MAV_PARAM_TYPE_INT32,
        MavParamType.MAV_PARAM_TYPE_UINT8,
        MavParamType.MAV_PARAM_TYPE_UINT16,
        MavParamType.MAV_PARAM_TYPE_UINT32,
    ):
        return int(raw)
    return float(raw)


async def upload_mission(ctx: GcsContext) -> None:
    plan = build_sample_mission(
        target_system=ctx.target_system,
        target_component=ctx.target_component,
    )
    cancel = MavlinkCancellationToken()
    ctx.operation_cancel = cancel

    print(f"[mission] uploading {len(plan)} hardcoded items...")
    result = await ctx.mission.upload(
        plan,
        cancel=cancel,
        on_progress=lambda sent, total, item: print(
            f"[mission upload] {sent}/{total} seq={item.seq} "
            f"{describe_mission_item(item)}"
        ),
    )
    print(f"[mission] upload finished: {result.name}")


async def download_mission(ctx: GcsContext) -> None:
    cancel = MavlinkCancellationToken()
    ctx.operation_cancel = cancel

    items = await ctx.mission.download(
        cancel=cancel,
        on_progress=lambda received, total, item: print(
            f"[mission download] {received}/{total} {describe_mission_item(item)}"
        ),
    )
    print("[mission] on vehicle:")
    for item in items:
        print(f"  {describe_mission_item(item)}")


async def clear_mission(ctx: GcsContext) -> None:
    print("[mission] sending MISSION_CLEAR_ALL...")
    result = await ctx.mission.clear()
    print(f"[mission] clear result: {result.name}")


async def set_mission_current(ctx: GcsContext, parts: list[str]) -> None:
    if len(parts) < 2:
        print("Usage: ms <seq>")
        return

    seq = int(parts[1])
    print(f"[mission] set current seq={seq} (mission + command)...")
    result = await ctx.mission.set_current_with_command(
        seq,
        command=ctx.command,
    )
    ack_name = result.command_ack.result.name if result.command_ack is not None else "n/a"
    print(f"[mission] seq={result.sequence} command ack={ack_name}")


async def request_message(ctx: GcsContext, parts: list[str]) -> None:
    if len(parts) < 2:
        print(f"Usage: rm <msgId>  (e.g. rm {Attitude.MSG_ID} for ATTITUDE)")
        return

    msg_id = int(parts[1])
    print(f"[command] REQUEST_MESSAGE id={msg_id}")
    ack = await ctx.command.request_message(msg_id)
    print(f"[command] ack: {ack.result.name}")

    if msg_id == Attitude.MSG_ID:
        print("[telemetry] waiting for ATTITUDE...")
        attitude = await ctx.session.wait_for_message_type(
            Attitude,
            from_system_id=ctx.target_system,
            timeout=timedelta(seconds=5),
        )
        print(
            f"[telemetry] roll={attitude.roll} pitch={attitude.pitch} yaw={attitude.yaw}"
        )


async def set_message_interval(ctx: GcsContext, parts: list[str]) -> None:
    if len(parts) < 3:
        print("Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)")
        return

    msg_id = int(parts[1])
    interval_us = int(parts[2])
    print(f"[command] SET_MESSAGE_INTERVAL id={msg_id} interval={interval_us} us")
    if interval_us == 0:
        ack = await ctx.command.stop_message_interval(msg_id)
    else:
        ack = await ctx.command.set_message_interval(msg_id, interval_us)
    print(f"[command] ack: {ack.result.name}")


async def stream_attitude(ctx: GcsContext, parts: list[str]) -> None:
    seconds = int(parts[1]) if len(parts) >= 2 else 5
    print(
        f"[telemetry] streaming ATTITUDE for {seconds}s (subscribe + interval)..."
    )

    await ctx.command.set_message_interval(Attitude.MSG_ID, 100000)

    count = 0

    def on_attitude(attitude: Attitude, _frame) -> None:
        nonlocal count
        count += 1
        print(
            f"[attitude] #{count} roll={attitude.roll:.3f} "
            f"pitch={attitude.pitch:.3f} yaw={attitude.yaw:.3f}"
        )

    subscription = ctx.session.listen_message(
        Attitude,
        on_attitude,
        from_system_id=ctx.target_system,
    )

    await asyncio.sleep(seconds)
    subscription.cancel()
    await ctx.command.stop_message_interval(Attitude.MSG_ID)
    print(f"[telemetry] received {count} ATTITUDE messages")


async def arm(ctx: GcsContext, parts: list[str]) -> None:
    force = len(parts) >= 2 and parts[1].lower() == "force"
    print(f"[command] ARM{' (force)' if force else ''}...")
    ack = await ctx.command.arm(force=force)
    print(f"[command] ack: {ack.result.name}")


async def disarm(ctx: GcsContext, parts: list[str]) -> None:
    force = len(parts) >= 2 and parts[1].lower() == "force"
    print(f"[command] DISARM{' (force)' if force else ''}...")
    ack = await ctx.command.disarm(force=force)
    print(f"[command] ack: {ack.result.name}")


async def return_to_launch(ctx: GcsContext) -> None:
    print("[command] RETURN_TO_LAUNCH...")
    ack = await ctx.command.return_to_launch()
    print(f"[command] ack: {ack.result.name}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nInterrupted.")
