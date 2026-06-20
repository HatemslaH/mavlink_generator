from __future__ import annotations

import sys

from serial.tools import list_ports


def pick_serial_port() -> str:
    """List available serial ports and read a selection from stdin."""
    ports = list(list_ports.comports())
    if not ports:
        raise RuntimeError("No serial ports found. Connect SITL or a USB adapter.")

    print()
    print("Available serial ports:")
    for index, info in enumerate(ports):
        details = " — ".join(
            part for part in (info.description, info.manufacturer) if part
        )
        device = info.device
        print(f"  [{index}] {device}{f' ({details})' if details else ''}")
    print()
    print(f"Select port [0-{len(ports) - 1}]: ", end="", flush=True)

    line = sys.stdin.readline()
    if line is None:
        raise RuntimeError("Port selection required")
    line = line.strip()
    if not line:
        raise RuntimeError("Port selection required")

    try:
        selected = int(line)
    except ValueError as exc:
        raise RuntimeError(f"Invalid port selection: {line}") from exc

    if selected < 0 or selected >= len(ports):
        raise RuntimeError(f"Invalid port selection: {line}")

    port_name = ports[selected].device
    print(f"Selected {port_name}")
    return port_name


def parse_baud_rate(args: list[str], default_baud: int = 57600) -> int:
    """Parse ``--baud <rate>`` from CLI arguments (default 57600)."""
    for index in range(len(args) - 1):
        if args[index] == "--baud":
            try:
                value = int(args[index + 1])
            except ValueError as exc:
                raise ValueError(f"Invalid --baud value: {args[index + 1]}") from exc
            if value <= 0:
                raise ValueError(f"Invalid --baud value: {args[index + 1]}")
            return value
    return default_baud
