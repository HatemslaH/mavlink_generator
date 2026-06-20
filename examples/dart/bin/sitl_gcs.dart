// ignore_for_file: avoid_print

import 'dart:async';
import 'dart:io';

import 'package:mavlink/mavlink_protocols.dart';
import 'package:mavlink_sitl_gcs/gcs_context.dart';
import 'package:mavlink_sitl_gcs/port_picker.dart';
import 'package:mavlink_sitl_gcs/sample_mission.dart';
import 'package:mavlink_sitl_gcs/serial_link.dart';

Future<void> main(List<String> args) async {
  final baudRate = parseBaudRate(args);
  final portName = await pickSerialPort();

  stdout.writeln();
  stdout.writeln('Opening $portName @ $baudRate baud...');

  final dialect = MavlinkDialectRt_rc();
  final link = SerialMavlinkLink.open(portName, baudRate: baudRate);
  final gcs = MavlinkGcs.connect(
    dialect: dialect,
    link: link,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  );

  gcs.start();
  stdout.writeln('Publishing GCS heartbeats, waiting for vehicle...');

  MavlinkVehicleClient client;
  try {
    client = await gcs.waitForVehicle(
      excludeSystemIds: {gcsSystemId},
      timeout: const Duration(seconds: 60),
    );
  } on MavlinkTimeoutException {
    throw StateError(
      'No vehicle heartbeat within 60 s. Check port, baud (current: $baudRate; try --baud 115200), and SITL.',
    );
  }

  final vehicle = client.vehicle;
  final vehicleState = gcs.heartbeatMonitor.stateFor(vehicle);
  stdout.writeln('Vehicle online: $vehicle');
  if (vehicleState != null) {
    stdout.writeln(
      '  type=${vehicleState.heartbeat.type.name} '
      'autopilot=${vehicleState.heartbeat.autopilot.name} '
      'status=${vehicleState.heartbeat.systemStatus.name}',
    );
  }

  final ctx = GcsContext(gcs: gcs, vehicle: vehicle, client: client);

  stdout.writeln();
  stdout.writeln('=== Phase 2: parameter sync ===');
  await _fetchAllParameters(ctx);

  stdout.writeln();
  stdout.writeln('=== Interactive CLI ===');
  await _runCli(ctx);

  stdout.writeln('Shutting down...');
  ctx.operationCancel?.cancel();
  await gcs.close();
}

Future<void> _fetchAllParameters(GcsContext ctx) async {
  final cancel = MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  stdout.writeln('[parameters] waiting for PARAM_VALUE stream...');
  final entries = await ctx.parameters.fetchAll(
    cancel: cancel,
    onProgress: (entry, received, expected) {
      if (received == 1) {
        stdout.writeln('[parameters] expecting $expected parameters');
      }
      stdout.writeln(
        '[parameters] $received/$expected '
        '${entry.id}=${entry.value} (${entry.type.name})',
      );
    },
  );
  stdout.writeln('[parameters] complete (${entries.length} total, cache=${ctx.parameters.cache.length})');
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
        case 'cancel':
          _cancelOperation(ctx);
        case 'p':
        case 'params':
          await _fetchAllParameters(ctx);
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
        case 'att':
          await _streamAttitude(ctx, parts);
        case 'arm':
          await _arm(ctx, parts);
        case 'disarm':
          await _disarm(ctx, parts);
        case 'rtl':
          await _returnToLaunch(ctx);
        default:
          stdout.writeln('Unknown command: $command (type help)');
      }
    } on MavlinkCancelledException {
      stdout.writeln('Operation cancelled.');
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
  stdout.writeln('  cancel            Cancel in-flight params/mission operation');
  stdout.writeln('  params            Request full parameter list (with progress)');
  stdout.writeln('  pr <name>         Read one parameter by name');
  stdout.writeln('  pw <name> <value> Write parameter (type from cache or REAL32)');
  stdout.writeln('  mu                Upload hardcoded sample mission');
  stdout.writeln('  md                Download mission from vehicle');
  stdout.writeln('  mc                Clear onboard mission');
  stdout.writeln('  ms <seq>          Set active mission item (mission + command)');
  stdout.writeln('  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)');
  stdout.writeln('  si <msgId> <us>   Set message interval (microseconds)');
  stdout.writeln('  att [seconds]     Stream ATTITUDE via onMessage (default 5 s)');
  stdout.writeln('  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)');
  stdout.writeln('  disarm [force]    Disarm motors');
  stdout.writeln('  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH');
  stdout.writeln('  quit              Exit');
}

void _cancelOperation(GcsContext ctx) {
  final token = ctx.operationCancel;
  if (token == null || token.isCancelled) {
    stdout.writeln('[cancel] no active cancellable operation');
    return;
  }
  token.cancel();
  stdout.writeln('[cancel] signalled');
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
  final cachedType = ctx.parameters.typeForName(name);
  final type = cachedType ?? MavParamType.mavParamTypeReal32;
  final value = _parseParamValue(rawValue, type);

  stdout.writeln('[parameters] writing $name=$value ($type)...');
  final entry = await ctx.parameters.writeByName(name, value);
  stdout.writeln('[parameters] ack $name=${entry.value} (${entry.type.name})');
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
  final cancel = MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  stdout.writeln('[mission] uploading ${plan.length} hardcoded items...');
  final result = await ctx.mission.upload(
    plan,
    cancel: cancel,
    onProgress: (sent, total, item) {
      stdout.writeln(
        '[mission upload] $sent/$total seq=${item.seq} '
        '${describeMissionItem(item)}',
      );
    },
  );
  stdout.writeln('[mission] upload finished: ${result.name}');
}

Future<void> _downloadMission(GcsContext ctx) async {
  final cancel = MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  final items = await ctx.mission.download(
    cancel: cancel,
    onProgress: (received, total, item) {
      stdout.writeln(
        '[mission download] $received/$total '
        '${describeMissionItem(item)}',
      );
    },
  );
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
  stdout.writeln('[mission] set current seq=$seq (mission + command)...');
  final result = await ctx.mission.setCurrentWithCommand(
    seq,
    command: ctx.command,
  );
  stdout.writeln(
    '[mission] seq=${result.sequence} '
    'command ack=${result.commandAck?.result.name ?? 'n/a'}',
  );
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
    stdout.writeln('Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)');
    return;
  }

  final msgId = int.parse(parts[1]);
  final intervalUs = int.parse(parts[2]);
  stdout.writeln('[command] SET_MESSAGE_INTERVAL id=$msgId interval=$intervalUs us');
  final ack = intervalUs == 0
      ? await ctx.command.stopMessageInterval(msgId)
      : await ctx.command.setMessageInterval(msgId, intervalUs);
  stdout.writeln('[command] ack: ${ack.result.name}');
}

Future<void> _streamAttitude(GcsContext ctx, List<String> parts) async {
  final seconds = parts.length >= 2 ? int.parse(parts[1]) : 5;
  stdout.writeln('[telemetry] streaming ATTITUDE for ${seconds}s (subscribe + interval)...');

  await ctx.command.setMessageInterval(Attitude.msgId, 100000);

  var count = 0;
  final subscription = ctx.session.listenMessage<Attitude>(
    (attitude, frame) {
      count++;
      stdout.writeln(
        '[attitude] #$count roll=${attitude.roll.toStringAsFixed(3)} '
        'pitch=${attitude.pitch.toStringAsFixed(3)} '
        'yaw=${attitude.yaw.toStringAsFixed(3)}',
      );
    },
    fromSystemId: ctx.targetSystem,
  );

  await Future<void>.delayed(Duration(seconds: seconds));
  subscription.cancel();
  await ctx.command.stopMessageInterval(Attitude.msgId);
  stdout.writeln('[telemetry] received $count ATTITUDE messages');
}

Future<void> _arm(GcsContext ctx, List<String> parts) async {
  final force = parts.length >= 2 && parts[1].toLowerCase() == 'force';
  stdout.writeln('[command] ARM${force ? ' (force)' : ''}...');
  final ack = await ctx.command.arm(force: force);
  stdout.writeln('[command] ack: ${ack.result.name}');
}

Future<void> _disarm(GcsContext ctx, List<String> parts) async {
  final force = parts.length >= 2 && parts[1].toLowerCase() == 'force';
  stdout.writeln('[command] DISARM${force ? ' (force)' : ''}...');
  final ack = await ctx.command.disarm(force: force);
  stdout.writeln('[command] ack: ${ack.result.name}');
}

Future<void> _returnToLaunch(GcsContext ctx) async {
  stdout.writeln('[command] RETURN_TO_LAUNCH...');
  final ack = await ctx.command.returnToLaunch();
  stdout.writeln('[command] ack: ${ack.result.name}');
}
