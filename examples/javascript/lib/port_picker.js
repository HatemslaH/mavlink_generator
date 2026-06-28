import { SerialPort } from 'serialport';
import * as readline from 'node:readline/promises';
import { stdin, stdout } from 'node:process';

/** Lists available serial ports and reads a selection from stdin. */
export async function pickSerialPort() {
  const portInfos = await SerialPort.list();
  if (portInfos.length === 0) {
    throw new Error('No serial ports found. Connect SITL or a USB adapter.');
  }

  console.log();
  console.log('Available serial ports:');
  for (let index = 0; index < portInfos.length; index++) {
    const info = portInfos[index];
    const details = [info.friendlyName, info.manufacturer].filter(Boolean).join(' — ');
    console.log(`  [${index}] ${info.path}${details ? ` (${details})` : ''}`);
  }
  console.log();

  const rl = readline.createInterface({ input: stdin, output: stdout });
  const line = (await rl.question(`Select port [0-${portInfos.length - 1}]: `)).trim();
  rl.close();

  if (!line) {
    throw new Error('Port selection required');
  }

  const selected = Number.parseInt(line, 10);
  if (Number.isNaN(selected) || selected < 0 || selected >= portInfos.length) {
    throw new Error(`Invalid port selection: ${line}`);
  }

  const portName = portInfos[selected].path;
  console.log(`Selected ${portName}`);
  return portName;
}

/** Parse `--baud <rate>` from CLI arguments (default 57600). */
export function parseBaudRate(args, defaultBaud = 57600) {
  for (let index = 0; index < args.length - 1; index++) {
    if (args[index] === '--baud') {
      const value = Number.parseInt(args[index + 1], 10);
      if (Number.isNaN(value) || value <= 0) {
        throw new Error(`Invalid --baud value: ${args[index + 1]}`);
      }
      return value;
    }
  }
  return defaultBaud;
}
