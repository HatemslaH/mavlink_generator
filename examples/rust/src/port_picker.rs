//! Serial port discovery and CLI baud-rate parsing.

use std::io::{self, Write};

/// Lists available serial ports and reads a selection from stdin.
pub fn pick_serial_port() -> io::Result<String> {
    let ports = serialport::available_ports().map_err(|error| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("failed to enumerate serial ports: {error}"),
        )
    })?;

    if ports.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No serial ports found. Connect SITL or a USB adapter.",
        ));
    }

    println!();
    println!("Available serial ports:");
    for (index, port) in ports.iter().enumerate() {
        let details = match &port.port_type {
            serialport::SerialPortType::UsbPort(info) => {
                let mut parts = Vec::new();
                if let Some(manufacturer) = &info.manufacturer {
                    parts.push(manufacturer.clone());
                }
                if let Some(product) = &info.product {
                    parts.push(product.clone());
                }
                parts.join(" — ")
            }
            serialport::SerialPortType::BluetoothPort => "Bluetooth".to_string(),
            serialport::SerialPortType::PciPort => "PCI".to_string(),
            serialport::SerialPortType::Unknown => String::new(),
        };
        if details.is_empty() {
            println!("  [{index}] {}", port.port_name);
        } else {
            println!("  [{index}] {} ({details})", port.port_name);
        }
    }
    println!();
    print!("Select port [0-{}]: ", ports.len() - 1);
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Port selection required",
        ));
    }

    let selected: usize = line.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid port selection: {line}"),
        )
    })?;
    if selected >= ports.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid port selection: {line}"),
        ));
    }

    let port_name = ports[selected].port_name.clone();
    println!("Selected {port_name}");
    Ok(port_name)
}

/// Parse `--baud <rate>` from CLI arguments (default 57600).
pub fn parse_baud_rate(args: &[String], default_baud: u32) -> Result<u32, String> {
    for index in 0..args.len().saturating_sub(1) {
        if args[index] == "--baud" {
            let value = args[index + 1]
                .parse::<u32>()
                .map_err(|_| format!("Invalid --baud value: {}", args[index + 1]))?;
            if value == 0 {
                return Err(format!("Invalid --baud value: {}", args[index + 1]));
            }
            return Ok(value);
        }
    }
    Ok(default_baud)
}
