// ignore_for_file: avoid_print

import 'dart:async';
import 'dart:io';

import 'package:mavlink/mavlink_protocols.dart';
import 'package:mavlink_sitl_gcs/gcs_context.dart';
import 'package:mavlink_sitl_gcs/port_picker.dart';
import 'package:mavlink_sitl_gcs/protocol_progress.dart';
import 'package:mavlink_sitl_gcs/sample_mission.dart';
import 'package:mavlink_sitl_gcs/serial_link.dart';

Future<void> main(List<String> args) async {
  final baudRate = parseBaudRate(args);
  final portName = await pickSerialPort();

  stdout.writeln();
  stdout.writeln('Opening $portName @ $baudRate baud...');

  final dialect = MavlinkDialectRt_rc();
  final link = SerialMavlinkLink.open(portName, baudRate: baudRate);

  final session = MavlinkSession(
    dialect: dialect,
    link: link,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  );

  final heartbeatPublisher = HeartbeatPublisher(
    session: session,
    heartbeat: HeartbeatTemplates.gcs(mavlinkVersion: dialect.version),
    interval: const Duration(seconds: 1),
  );

  final heartbeatMonitor = HeartbeatMonitor(
    session: session,
    timeout: const Duration(seconds: 3),
  );

  heartbeatMonitor.start();
  heartbeatPublisher.start();

  stdout.writeln('Publishing GCS heartbeats, waiting for vehicle...');

  final vehicle = await _waitForVehicle(heartbeatMonitor, baudRate: baudRate);
  final vehicleState = heartbeatMonitor.stateFor(vehicle);
  stdout.writeln('Vehicle online: $vehicle');
  if (vehicleState != null) {
    stdout.writeln(
      '  type=${vehicleState.heartbeat.type.name} '
      'autopilot=${vehicleState.heartbeat.autopilot.name} '
      'status=${vehicleState.heartbeat.systemStatus.name}',
    );
  }

  final ctx = GcsContext(
    session: session,
    dialect: dialect,
    vehicle: vehicle,
    heartbeatMonitor: heartbeatMonitor,
    heartbeatPublisher: heartbeatPublisher,
  );

  stdout.writeln();
  stdout.writeln('=== Phase 2: parameter sync ===');
  await fetchAllParametersWithProgress(ctx);

  stdout.writeln();
  stdout.writeln('=== Interactive CLI ===');
  await _runCli(ctx);

  stdout.writeln('Shutting down...');
  heartbeatPublisher.stop();
  await heartbeatMonitor.stop();
  await session.close();
}

Future<MavlinkNode> _waitForVehicle(
  HeartbeatMonitor monitor, {
  required int baudRate,
}) async {
  final completer = Completer<MavlinkNode>();

  final sub = monitor.onConnected.listen((node) {
    if (node.systemId == gcsSystemId) {
      return;
    }
    if (!completer.isCompleted) {
      completer.complete(node);
    }
  });

  try {
    return await completer.future.timeout(const Duration(seconds: 60));
  } on TimeoutException {
    throw StateError(
      'No vehicle heartbeat within 60 s. Check port, baud (current: $baudRate; try --baud 115200), and SITL.',
    );
  } finally {
    await sub.cancel();
  }
}

Future<void> _runCli(GcsContext ctx) async {
  _printHelp();

  while (true) {
    stdout.write('gcs> ');
    final line = stdin.readLineSync();
    if (line == null) {
      break;
    }

    final trimmed = line.trim();
    if (trimmed.isEmpty) {
      continue;
    }

    final parts = trimmed.split(RegExp(r'\s+'));
    final command = parts.first.toLowerCase();

    try {
      switch (command) {
        case 'h':
        case 'help':
          _printHelp();
        case 'q':
        case 'quit':
        case 'exit':
          return;
        case 'hb':
          _printHeartbeatStatus(ctx);
        case 'p':
        case 'params':
          await fetchAllParametersWithProgress(ctx);
        case 'pr':
          await _readParameter(ctx, parts);
        case 'pw':
          await _writeParameter(ctx, parts);
        case 'mu':
          await _uploadMission(ctx);
        case 'md':
          await _downloadMission(ctx);
        case 'mc':
          await _clearMission(ctx);
        case 'ms':
          await _setMissionCurrent(ctx, parts);
        case 'rm':
          await _requestMessage(ctx, parts);
        case 'si':
          await _setMessageInterval(ctx, parts);
        default:
          stdout.writeln('Unknown command: $command (type help)');
      }
    } on Exception catch (error) {
      stdout.writeln('Error: $error');
    }

    stdout.writeln();
  }
}

void _printHelp() {
  stdout.writeln('Commands:');
  stdout.writeln('  help              Show this help');
  stdout.writeln('  hb                Heartbeat / link status');
  stdout.writeln('  params            Request full parameter list');
  stdout.writeln('  pr <name>         Read one parameter by name');
  stdout.writeln('  pw <name> <value> Write parameter (uses cached type or REAL32)');
  stdout.writeln('  mu                Upload hardcoded sample mission');
  stdout.writeln('  md                Download mission from vehicle');
  stdout.writeln('  mc                Clear onboard mission');
  stdout.writeln('  ms <seq>          Set active mission item (COMMAND + protocol)');
  stdout.writeln('  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)');
  stdout.writeln('  si <msgId> <us>   Set message interval (microseconds)');
  stdout.writeln('  quit              Exit');
}

void _printHeartbeatStatus(GcsContext ctx) {
  final node = ctx.vehicle;
  final online = ctx.heartbeatMonitor.isOnline(node);
  final state = ctx.heartbeatMonitor.stateFor(node);

  stdout.writeln('[heartbeat] vehicle $node online=$online');
  if (state != null) {
    stdout.writeln(
      '  last=${state.age.inMilliseconds}ms ago '
      'type=${state.heartbeat.type.name} '
      'status=${state.heartbeat.systemStatus.name}',
    );
  } else {
    stdout.writeln('  no heartbeat received yet');
  }
}

Future<void> _readParameter(GcsContext ctx, List<String> parts) async {
  if (parts.length < 2) {
    stdout.writeln('Usage: pr <name>');
    return;
  }

  final name = parts[1];
  stdout.writeln('[parameters] reading $name...');
  final entry = await ctx.parameters.readByName(name);
  stdout.writeln(
    '[parameters] $name=${entry.value} (${entry.type.name}, '
    'index ${entry.index}/${entry.count})',
  );
}

Future<void> _writeParameter(GcsContext ctx, List<String> parts) async {
  if (parts.length < 3) {
    stdout.writeln('Usage: pw <name> <value>');
    return;
  }

  final name = parts[1];
  final rawValue = parts[2];
  ParamEntry? cached;
  for (final entry in ctx.cachedParameters) {
    if (entry.id == name) {
      cached = entry;
      break;
    }
  }
  final type = cached?.type ?? MavParamType.mavParamTypeReal32;
  final value = _parseParamValue(rawValue, type);

  stdout.writeln('[parameters] writing $name=$value ($type)...');
  final entry = await ctx.parameters.write(name: name, value: value, type: type);
  stdout.writeln('[parameters] ack $name=${entry.value}');
}

num _parseParamValue(String raw, MavParamType type) {
  return switch (type) {
    MavParamType.mavParamTypeInt8 ||
    MavParamType.mavParamTypeInt16 ||
    MavParamType.mavParamTypeInt32 ||
    MavParamType.mavParamTypeUint8 ||
    MavParamType.mavParamTypeUint16 ||
    MavParamType.mavParamTypeUint32 =>
      int.parse(raw),
    _ => double.parse(raw),
  };
}

Future<void> _uploadMission(GcsContext ctx) async {
  final plan = buildSampleMission(
    targetSystem: ctx.targetSystem,
    targetComponent: ctx.targetComponent,
  );
  stdout.writeln('[mission] uploading ${plan.length} hardcoded items...');
  final result = await uploadMissionWithProgress(ctx, plan);
  stdout.writeln('[mission] upload finished: ${result.name}');
}

Future<void> _downloadMission(GcsContext ctx) async {
  final items = await downloadMissionWithProgress(ctx);
  stdout.writeln('[mission] on vehicle:');
  for (final item in items) {
    stdout.writeln('  ${describeMissionItem(item)}');
  }
}

Future<void> _clearMission(GcsContext ctx) async {
  stdout.writeln('[mission] sending MISSION_CLEAR_ALL...');
  final result = await ctx.mission.clear();
  stdout.writeln('[mission] clear result: ${result.name}');
}

Future<void> _setMissionCurrent(GcsContext ctx, List<String> parts) async {
  if (parts.length < 2) {
    stdout.writeln('Usage: ms <seq>');
    return;
  }

  final seq = int.parse(parts[1]);
  stdout.writeln('[mission] MISSION_SET_CURRENT seq=$seq');
  await ctx.mission.setCurrent(seq);

  stdout.writeln('[command] MAV_CMD_DO_SET_MISSION_CURRENT seq=$seq');
  final ack = await ctx.command.setMissionCurrent(seq);
  stdout.writeln('[command] ack: ${ack.result.name}');
}

Future<void> _requestMessage(GcsContext ctx, List<String> parts) async {
  if (parts.length < 2) {
    stdout.writeln('Usage: rm <msgId>  (e.g. rm ${Attitude.msgId} for ATTITUDE)');
    return;
  }

  final msgId = int.parse(parts[1]);
  stdout.writeln('[command] REQUEST_MESSAGE id=$msgId');
  final ack = await ctx.command.requestMessage(msgId);
  stdout.writeln('[command] ack: ${ack.result.name}');

  if (msgId == Attitude.msgId) {
    stdout.writeln('[telemetry] waiting for ATTITUDE...');
    final attitude = await ctx.session.waitForMessageType<Attitude>(
      fromSystemId: ctx.targetSystem,
      timeout: const Duration(seconds: 5),
    );
    stdout.writeln(
      '[telemetry] roll=${attitude.roll} pitch=${attitude.pitch} yaw=${attitude.yaw}',
    );
  }
}

Future<void> _setMessageInterval(GcsContext ctx, List<String> parts) async {
  if (parts.length < 3) {
    stdout.writeln('Usage: si <msgId> <interval_us>  (100000 = 10 Hz)');
    return;
  }

  final msgId = int.parse(parts[1]);
  final intervalUs = int.parse(parts[2]);
  stdout.writeln('[command] SET_MESSAGE_INTERVAL id=$msgId interval=$intervalUs us');
  final ack = await ctx.command.setMessageInterval(msgId, intervalUs);
  stdout.writeln('[command] ack: ${ack.result.name}');
}
