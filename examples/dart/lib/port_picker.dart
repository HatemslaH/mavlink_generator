import 'dart:io';

import 'package:serial_port_win32/serial_port_win32.dart';

/// Lists available COM ports and reads a selection from stdin.
Future<String> pickSerialPort() async {
  if (!Platform.isWindows) {
    throw UnsupportedError(
      'This example uses serial_port_win32 and currently supports Windows only.',
    );
  }

  final portInfos = SerialPort.getPortsWithFullMessages();
  if (portInfos.isEmpty) {
    throw StateError('No serial ports found. Connect SITL or a USB adapter.');
  }

  stdout.writeln();
  stdout.writeln('Available serial ports:');
  for (var index = 0; index < portInfos.length; index++) {
    final info = portInfos[index];
    final details = [
      if (info.friendlyName.isNotEmpty) info.friendlyName,
      if (info.manufactureName.isNotEmpty) info.manufactureName,
    ].join(' — ');
    stdout.writeln(
      '  [$index] ${info.portName}${details.isEmpty ? '' : ' ($details)'}',
    );
  }
  stdout.writeln();
  stdout.write('Select port [0-${portInfos.length - 1}]: ');

  final line = stdin.readLineSync()?.trim();
  if (line == null || line.isEmpty) {
    throw StateError('Port selection required');
  }

  final selected = int.tryParse(line);
  if (selected == null || selected < 0 || selected >= portInfos.length) {
    throw StateError('Invalid port selection: $line');
  }

  final portName = portInfos[selected].portName;
  stdout.writeln('Selected $portName');
  return portName;
}

/// Parse `--baud <rate>` from CLI arguments (default 57600).
int parseBaudRate(List<String> args, {int defaultBaud = 57600}) {
  for (var index = 0; index < args.length - 1; index++) {
    if (args[index] == '--baud') {
      final value = int.tryParse(args[index + 1]);
      if (value == null || value <= 0) {
        throw ArgumentError('Invalid --baud value: ${args[index + 1]}');
      }
      return value;
    }
  }
  return defaultBaud;
}
