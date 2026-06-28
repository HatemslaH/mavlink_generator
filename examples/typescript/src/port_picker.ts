import { createInterface } from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';
import { SerialPort } from 'serialport';

/** Lists available serial ports and reads a selection from stdin. */
export async function pickSerialPort(): Promise<string> {
  const ports = await SerialPort.list();
  if (ports.length === 0) {
    throw new Error(
      'No serial ports found. Connect SITL or a USB adapter.',
    );
  }

  console.log();
  console.log('Available serial ports:');
  for (let index = 0; index < ports.length; index++) {
    const info = ports[index]!;
    const details = [info.manufacturer, info.pnpId, info.productId]
      .filter((value) => value !== undefined && value.length > 0)
      .join(' — ');
    console.log(
      `  [${index}] ${info.path}${details.length === 0 ? '' : ` (${details})`}`,
    );
  }
  console.log();

  const rl = createInterface({ input, output });
  try {
    const line = await rl.question(
      `Select port [0-${ports.length - 1}]: `,
    );
    const trimmed = line.trim();
    if (trimmed.length === 0) {
      throw new Error('Port selection required');
    }

    const selected = Number.parseInt(trimmed, 10);
    if (
      Number.isNaN(selected) ||
      selected < 0 ||
      selected >= ports.length
    ) {
      throw new Error(`Invalid port selection: ${trimmed}`);
    }

    const path = ports[selected]!.path;
    console.log(`Selected ${path}`);
    return path;
  } finally {
    rl.close();
  }
}

/** Parse `--baud <rate>` or a bare numeric rate from CLI arguments (default 57600). */
export function parseBaudRate(
  args: readonly string[],
  defaultBaud = 57600,
): number {
  for (let index = 0; index < args.length - 1; index++) {
    if (args[index] === '--baud') {
      const value = Number.parseInt(args[index + 1]!, 10);
      if (Number.isNaN(value) || value <= 0) {
        throw new Error(`Invalid --baud value: ${args[index + 1]}`);
      }
      return value;
    }
  }

  if (args.length > 0) {
    const value = Number.parseInt(args[0]!, 10);
    if (!Number.isNaN(value) && value > 0) {
      return value;
    }
  }

  return defaultBaud;
}
