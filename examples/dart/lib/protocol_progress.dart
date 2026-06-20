import 'dart:io' show stdout;

import 'package:mavlink/mavlink_protocols.dart';

import 'gcs_context.dart';
import 'sample_mission.dart';

/// Request the full onboard parameter set with progress lines on stdout.
Future<List<ParamEntry>> fetchAllParametersWithProgress(GcsContext ctx) async {
  final session = ctx.session;
  final targetSystem = ctx.targetSystem;

  await session.send(
    ParamRequestList(
      targetSystem: ctx.targetSystem,
      targetComponent: ctx.targetComponent,
    ),
  );

  final entries = <ParamEntry>[];
  var expectedCount = -1;
  final seenIndices = <int>{};

  stdout.writeln('[parameters] waiting for PARAM_VALUE stream...');

  while (true) {
    final value = await session.waitForMessage(
      predicate: (message) {
        if (message is! ParamValue) {
          return false;
        }
        return !seenIndices.contains(message.paramIndex);
      },
      fromSystemId: targetSystem,
      timeout: expectedCount == -1
          ? ctx.parameters.requestTimeout
          : ctx.parameters.idleTimeout,
    );

    final paramValue = value as ParamValue;
    seenIndices.add(paramValue.paramIndex);

    if (expectedCount == -1) {
      expectedCount = paramValue.paramCount;
      stdout.writeln('[parameters] expecting $expectedCount parameters');
    }

    final entry = ParamEntry.fromParamValue(paramValue);
    entries.add(entry);

    stdout.writeln(
      '[parameters] ${entries.length}/$expectedCount '
      '${entry.id}=${entry.value} (${entry.type.name})',
    );

    if (entries.length >= expectedCount) {
      break;
    }
  }

  ctx.cachedParameters = entries;
  stdout.writeln('[parameters] complete (${entries.length} total)');
  return entries;
}

/// Upload [items] with per-item progress output.
Future<MavMissionResult> uploadMissionWithProgress(
  GcsContext ctx,
  List<MissionItemInt> items, {
  MavMissionType missionType = MavMissionType.mavMissionTypeMission,
}) async {
  final session = ctx.session;
  final plan = MissionItems.withSequentialSeq(items);

  stdout.writeln('[mission upload] sending MISSION_COUNT (${plan.length} items)');
  await session.send(
    MissionCount(
      count: plan.length,
      targetSystem: ctx.targetSystem,
      targetComponent: ctx.targetComponent,
      missionType: missionType,
    ),
  );

  for (final item in plan) {
    stdout.writeln(
      '[mission upload] waiting for request seq=${item.seq} '
      '(${describeMissionItem(item)})',
    );

    final request = await session.waitForMessage(
      predicate: (message) => _isItemRequest(message, item.seq, missionType),
      fromSystemId: ctx.targetSystem,
      timeout: ctx.mission.itemTimeout,
    );

    if (request is MissionRequestInt) {
      await session.send(item);
    } else if (request is MissionRequest) {
      await session.send(MissionItems.toLegacyItem(item));
    }

    stdout.writeln(
      '[mission upload] sent item ${item.seq + 1}/${plan.length}',
    );
  }

  stdout.writeln('[mission upload] waiting for MISSION_ACK...');
  final ack = await session.waitForMessageType<MissionAck>(
    fromSystemId: ctx.targetSystem,
    timeout: ctx.mission.operationTimeout,
  );

  stdout.writeln('[mission upload] result: ${ack.type.name}');
  return ack.type;
}

/// Download the vehicle mission with per-item progress output.
Future<List<MissionItemInt>> downloadMissionWithProgress(
  GcsContext ctx, {
  MavMissionType missionType = MavMissionType.mavMissionTypeMission,
}) async {
  final session = ctx.session;

  stdout.writeln('[mission download] requesting mission list...');
  await session.send(
    MissionRequestList(
      targetSystem: ctx.targetSystem,
      targetComponent: ctx.targetComponent,
      missionType: missionType,
    ),
  );

  final countMessage = await session.waitForMessageType<MissionCount>(
    fromSystemId: ctx.targetSystem,
    timeout: ctx.mission.operationTimeout,
  );

  stdout.writeln(
    '[mission download] vehicle reports ${countMessage.count} items',
  );

  final items = <MissionItemInt>[];

  for (var seq = 0; seq < countMessage.count; seq++) {
    stdout.writeln('[mission download] requesting seq=$seq...');
    await session.send(
      MissionRequestInt(
        seq: seq,
        targetSystem: ctx.targetSystem,
        targetComponent: ctx.targetComponent,
        missionType: missionType,
      ),
    );

    final itemMessage = await session.waitForMessage(
      predicate: (message) {
        if (message is MissionItemInt) {
          return message.seq == seq && message.missionType == missionType;
        }
        if (message is MissionItem) {
          return message.seq == seq && message.missionType == missionType;
        }
        return false;
      },
      fromSystemId: ctx.targetSystem,
      timeout: ctx.mission.itemTimeout,
    );

    final MissionItemInt item;
    if (itemMessage is MissionItemInt) {
      item = itemMessage;
    } else {
      item = MissionItems.fromLegacyItem(itemMessage as MissionItem);
    }

    items.add(item);
    stdout.writeln(
      '[mission download] received ${items.length}/${countMessage.count}: '
      '${describeMissionItem(item)}',
    );
  }

  await session.send(
    MissionAck(
      targetSystem: ctx.targetSystem,
      targetComponent: ctx.targetComponent,
      type: MavMissionResult.mavMissionAccepted,
      missionType: missionType,
    ),
  );

  stdout.writeln('[mission download] complete');
  return items;
}

bool _isItemRequest(MavlinkMessage message, int seq, MavMissionType missionType) {
  if (message is MissionRequestInt) {
    return message.seq == seq && message.missionType == missionType;
  }
  if (message is MissionRequest) {
    return message.seq == seq && message.missionType == missionType;
  }
  return false;
}