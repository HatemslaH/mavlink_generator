import 'dart:async';

import '../mavlink.dart';
import 'mavlink_session.dart';

/// Helpers for building and converting mission plan items.
class MissionItems {
  MissionItems._();

  /// Build a global waypoint using scaled integer lat/lon (MAVLink convention).
  static MissionItemInt waypoint({
    required int seq,
    required double latitude,
    required double longitude,
    required double altitude,
    required int targetSystem,
    required int targetComponent,
    MavCmd command = MavCmd.mavCmdNavWaypoint,
    MavFrame frame = MavFrame.mavFrameGlobalRelativeAltInt,
    MavMissionType missionType = MavMissionType.mavMissionTypeMission,
    float param1 = 0,
    float param2 = 0,
    float param3 = 0,
    float param4 = 0,
    uint8_t current = 0,
    uint8_t autocontinue = 1,
  }) {
    return MissionItemInt(
      param1: param1,
      param2: param2,
      param3: param3,
      param4: param4,
      x: (latitude * 1e7).round(),
      y: (longitude * 1e7).round(),
      z: altitude,
      seq: seq,
      command: command,
      targetSystem: targetSystem,
      targetComponent: targetComponent,
      frame: frame,
      current: current,
      autocontinue: autocontinue,
      missionType: missionType,
    );
  }

  /// Convert a [MissionItemInt] to the legacy [MissionItem] representation.
  static MissionItem toLegacyItem(MissionItemInt item) {
    return MissionItem(
      param1: item.param1,
      param2: item.param2,
      param3: item.param3,
      param4: item.param4,
      x: item.x / 1e7,
      y: item.y / 1e7,
      z: item.z,
      seq: item.seq,
      command: item.command,
      targetSystem: item.targetSystem,
      targetComponent: item.targetComponent,
      frame: item.frame,
      current: item.current,
      autocontinue: item.autocontinue,
      missionType: item.missionType,
    );
  }

  /// Convert a legacy [MissionItem] to [MissionItemInt].
  static MissionItemInt fromLegacyItem(MissionItem item) {
    return MissionItemInt(
      param1: item.param1,
      param2: item.param2,
      param3: item.param3,
      param4: item.param4,
      x: (item.x * 1e7).round(),
      y: (item.y * 1e7).round(),
      z: item.z,
      seq: item.seq,
      command: item.command,
      targetSystem: item.targetSystem,
      targetComponent: item.targetComponent,
      frame: item.frame,
      current: item.current,
      autocontinue: item.autocontinue,
      missionType: item.missionType,
    );
  }

  /// Re-number items sequentially starting from zero.
  static List<MissionItemInt> withSequentialSeq(List<MissionItemInt> items) {
    return [
      for (var index = 0; index < items.length; index++)
        MissionItemInt(
          param1: items[index].param1,
          param2: items[index].param2,
          param3: items[index].param3,
          param4: items[index].param4,
          x: items[index].x,
          y: items[index].y,
          z: items[index].z,
          seq: index,
          command: items[index].command,
          targetSystem: items[index].targetSystem,
          targetComponent: items[index].targetComponent,
          frame: items[index].frame,
          current: items[index].current,
          autocontinue: items[index].autocontinue,
          missionType: items[index].missionType,
        ),
    ];
  }
}

/// GCS-side MAVLink mission protocol client.
///
/// Supports upload, download, clear, and set-current operations per
/// https://mavlink.io/en/services/mission.html
class MissionProtocol {
  MissionProtocol({
    required this.session,
    required this.targetSystem,
    required this.targetComponent,
    this.itemTimeout = const Duration(seconds: 3),
    this.operationTimeout = const Duration(seconds: 10),
  });

  final MavlinkSession session;
  final int targetSystem;
  final int targetComponent;
  final Duration itemTimeout;
  final Duration operationTimeout;

  /// Upload a mission plan to the vehicle.
  Future<MavMissionResult> upload(
    List<MissionItemInt> items, {
    MavMissionType missionType = MavMissionType.mavMissionTypeMission,
  }) async {
    final plan = MissionItems.withSequentialSeq(items);

    await session.send(
      MissionCount(
        count: plan.length,
        targetSystem: targetSystem,
        targetComponent: targetComponent,
        missionType: missionType,
      ),
    );

    for (final item in plan) {
      final request = await session.waitForMessage(
        predicate: (message) => _isItemRequest(message, item.seq, missionType),
        fromSystemId: targetSystem,
        timeout: itemTimeout,
      );

      if (request is MissionRequestInt) {
        await session.send(item);
      } else if (request is MissionRequest) {
        await session.send(MissionItems.toLegacyItem(item));
      }
    }

    final ack = await session.waitForMessageType<MissionAck>(fromSystemId: targetSystem, timeout: operationTimeout);

    return ack.type;
  }

  /// Download a mission plan from the vehicle.
  Future<List<MissionItemInt>> download({MavMissionType missionType = MavMissionType.mavMissionTypeMission}) async {
    await session.send(
      MissionRequestList(targetSystem: targetSystem, targetComponent: targetComponent, missionType: missionType),
    );

    final countMessage = await session.waitForMessageType<MissionCount>(
      fromSystemId: targetSystem,
      timeout: operationTimeout,
    );

    final items = <MissionItemInt>[];

    for (var seq = 0; seq < countMessage.count; seq++) {
      await session.send(
        MissionRequestInt(
          seq: seq,
          targetSystem: targetSystem,
          targetComponent: targetComponent,
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
        fromSystemId: targetSystem,
        timeout: itemTimeout,
      );

      if (itemMessage is MissionItemInt) {
        items.add(itemMessage);
      } else if (itemMessage is MissionItem) {
        items.add(MissionItems.fromLegacyItem(itemMessage));
      }
    }

    await session.send(
      MissionAck(
        targetSystem: targetSystem,
        targetComponent: targetComponent,
        type: MavMissionResult.mavMissionAccepted,
        missionType: missionType,
      ),
    );

    return items;
  }

  /// Clear all mission items of the given type on the vehicle.
  Future<MavMissionResult> clear({MavMissionType missionType = MavMissionType.mavMissionTypeMission}) async {
    await session.send(
      MissionClearAll(targetSystem: targetSystem, targetComponent: targetComponent, missionType: missionType),
    );

    final ack = await session.waitForMessageType<MissionAck>(fromSystemId: targetSystem, timeout: operationTimeout);

    return ack.type;
  }

  /// Set the active mission item sequence number on the vehicle.
  Future<void> setCurrent(int seq) async {
    await session.send(MissionSetCurrent(seq: seq, targetSystem: targetSystem, targetComponent: targetComponent));
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
}

/// Vehicle-side mission protocol handler for embedding in autopilot code.
class MissionServer {
  MissionServer({
    required this.session,
    List<MissionItemInt>? initialMission,
    this.missionType = MavMissionType.mavMissionTypeMission,
  }) : _items = List<MissionItemInt>.from(initialMission ?? []) {
    _subscription = session.frames.listen(_onFrame);
  }

  final MavlinkSession session;
  final MavMissionType missionType;
  final List<MissionItemInt> _items;
  final Map<int, MissionItemInt> _incoming = {};
  int? _incomingCount;
  late final StreamSubscription<MavlinkFrame> _subscription;

  List<MissionItemInt> get items => List.unmodifiable(_items);

  Future<void> close() async {
    await _subscription.cancel();
  }

  void replaceMission(List<MissionItemInt> items) {
    _items
      ..clear()
      ..addAll(MissionItems.withSequentialSeq(items));
    _incoming.clear();
    _incomingCount = null;
  }

  Future<void> _onFrame(MavlinkFrame frame) async {
    final message = frame.message;

    if (message is MissionCount && _targetsUs(message)) {
      if (message.missionType != missionType) {
        return;
      }
      _incomingCount = message.count;
      _incoming.clear();
      if (message.count > 0) {
        await _requestUploadItem(frame, 0);
      } else {
        await _sendUploadAck(frame);
      }
      return;
    }

    if (message is MissionItemInt && _targetsUs(message)) {
      if (message.missionType != missionType) {
        return;
      }
      await _storeIncomingItem(frame, message);
      return;
    }

    if (message is MissionItem && _targetsUs(message)) {
      if (message.missionType != missionType) {
        return;
      }
      await _storeIncomingItem(frame, MissionItems.fromLegacyItem(message));
      return;
    }

    if (message is MissionRequestInt && _targetsUs(message)) {
      await _sendRequestedItem(frame, message.seq);
      return;
    }

    if (message is MissionRequest && _targetsUs(message)) {
      await _sendRequestedItem(frame, message.seq);
      return;
    }

    if (message is MissionRequestList && _targetsUs(message)) {
      if (message.missionType != missionType) {
        return;
      }
      await session.send(
        MissionCount(
          count: _items.length,
          targetSystem: frame.systemId,
          targetComponent: frame.componentId,
          missionType: missionType,
        ),
      );
      return;
    }

    if (message is MissionClearAll && _targetsUs(message)) {
      if (message.missionType != missionType) {
        return;
      }
      _items.clear();
      _incoming.clear();
      _incomingCount = null;
      await session.send(
        MissionAck(
          targetSystem: frame.systemId,
          targetComponent: frame.componentId,
          type: MavMissionResult.mavMissionAccepted,
          missionType: missionType,
        ),
      );
    }
  }

  Future<void> _storeIncomingItem(MavlinkFrame frame, MissionItemInt item) async {
    _incoming[item.seq] = item;
    final expected = _incomingCount;
    if (expected == null) {
      return;
    }

    if (_incoming.length < expected) {
      await _requestUploadItem(frame, item.seq + 1);
      return;
    }

    _items
      ..clear()
      ..addAll(List<MissionItemInt>.generate(expected, (index) => _incoming[index]!));
    _incoming.clear();
    _incomingCount = null;
    await _sendUploadAck(frame);
  }

  Future<void> _requestUploadItem(MavlinkFrame requestFrame, int seq) async {
    await session.send(
      MissionRequestInt(
        seq: seq,
        targetSystem: requestFrame.systemId,
        targetComponent: requestFrame.componentId,
        missionType: missionType,
      ),
    );
  }

  Future<void> _sendUploadAck(MavlinkFrame requestFrame) async {
    await session.send(
      MissionAck(
        targetSystem: requestFrame.systemId,
        targetComponent: requestFrame.componentId,
        type: MavMissionResult.mavMissionAccepted,
        missionType: missionType,
      ),
    );
  }

  Future<void> _sendRequestedItem(MavlinkFrame requestFrame, int seq) async {
    if (seq < 0 || seq >= _items.length) {
      await session.send(
        MissionAck(
          targetSystem: requestFrame.systemId,
          targetComponent: requestFrame.componentId,
          type: MavMissionResult.mavMissionInvalidSequence,
          missionType: missionType,
        ),
      );
      return;
    }

    await session.send(_items[seq]);
  }

  bool _targetsUs(MavlinkMessage message) {
    return switch (message) {
      MissionCount(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionItemInt(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionItem(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionRequestInt(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionRequest(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionRequestList(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      MissionClearAll(:final targetSystem, :final targetComponent) => _matchesTarget(targetSystem, targetComponent),
      _ => false,
    };
  }

  bool _matchesTarget(int targetSystem, int targetComponent) {
    if (targetSystem != session.systemId && targetSystem != 0) {
      return false;
    }
    if (targetComponent != session.componentId && targetComponent != MavComponent.mavCompIdAll.value) {
      return false;
    }
    return true;
  }
}
