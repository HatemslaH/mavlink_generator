import 'dart:async';

import '../mavlink.dart';
import 'mavlink_cancellation.dart';
import 'mavlink_session.dart';

/// MAVLink node identity (system + component).
class MavlinkNode {
  const MavlinkNode(this.systemId, this.componentId);

  final int systemId;
  final int componentId;

  @override
  bool operator ==(Object other) {
    return other is MavlinkNode && other.systemId == systemId && other.componentId == componentId;
  }

  @override
  int get hashCode => Object.hash(systemId, componentId);

  @override
  String toString() => 'MavlinkNode($systemId:$componentId)';
}

/// Last known heartbeat state for a remote node.
class TrackedHeartbeat {
  const TrackedHeartbeat({required this.node, required this.heartbeat, required this.receivedAt, required this.online});

  final MavlinkNode node;
  final Heartbeat heartbeat;
  final DateTime receivedAt;
  final bool online;

  Duration get age => DateTime.now().difference(receivedAt);
}

/// Tracks remote HEARTBEAT messages and reports connect / disconnect events.
///
/// Transport-agnostic: listens on [MavlinkSession.frames] only.
class HeartbeatMonitor {
  HeartbeatMonitor({required this.session, this.timeout = const Duration(seconds: 5), this.watch, this.watchSystemId});

  final MavlinkSession session;
  final Duration timeout;

  /// When set, only these nodes are tracked. When `null`, all heartbeats apply.
  final Set<MavlinkNode>? watch;

  /// When set, track every component on this system id.
  final int? watchSystemId;

  final _states = <MavlinkNode, TrackedHeartbeat>{};
  final _online = <MavlinkNode, bool>{};
  final _heartbeatController = StreamController<TrackedHeartbeat>.broadcast();
  final _connectedController = StreamController<MavlinkNode>.broadcast();
  final _disconnectedController = StreamController<MavlinkNode>.broadcast();

  StreamSubscription<MavlinkFrame>? _frameSubscription;
  Timer? _watchdogTimer;
  var _running = false;

  /// Emitted on every received (or recovered) heartbeat update.
  Stream<TrackedHeartbeat> get onHeartbeat => _heartbeatController.stream;

  /// Emitted when a watched node comes online (first heartbeat or recovery).
  Stream<MavlinkNode> get onConnected => _connectedController.stream;

  /// Emitted when a watched node times out without heartbeats.
  Stream<MavlinkNode> get onDisconnected => _disconnectedController.stream;

  /// Start monitoring. Safe to call only once; use [stop] before restarting.
  void start() {
    if (_running) {
      return;
    }
    _running = true;
    _frameSubscription = session.frames.listen(_onFrame);
    _watchdogTimer = Timer.periodic(Duration(milliseconds: timeout.inMilliseconds ~/ 3), (_) => _checkTimeouts());
  }

  /// Stop monitoring and release timers/subscriptions.
  Future<void> stop() async {
    if (!_running) {
      return;
    }
    _running = false;
    await _frameSubscription?.cancel();
    _frameSubscription = null;
    _watchdogTimer?.cancel();
    _watchdogTimer = null;
  }

  /// Returns the latest state for [node], or `null` if no heartbeat was seen.
  TrackedHeartbeat? stateFor(MavlinkNode node) => _states[node];

  /// Returns the latest state for [systemId]/[componentId].
  TrackedHeartbeat? stateForIds(int systemId, int componentId) {
    return stateFor(MavlinkNode(systemId, componentId));
  }

  /// Whether [node] is currently considered online.
  bool isOnline(MavlinkNode node) => _online[node] ?? false;

  /// Whether [systemId]/[componentId] is currently considered online.
  bool isOnlineIds(int systemId, int componentId) {
    return isOnline(MavlinkNode(systemId, componentId));
  }

  /// All nodes currently tracked as online.
  Iterable<MavlinkNode> get onlineNodes sync* {
    for (final entry in _online.entries) {
      if (entry.value) {
        yield entry.key;
      }
    }
  }

  /// Wait until the first online vehicle heartbeat is observed.
  ///
  /// [excludeSystemIds] skips GCS and other local identities (e.g. `{255}`).
  /// The monitor must already be [start]ed.
  Future<MavlinkNode> waitForVehicle({
    Set<int>? excludeSystemIds,
    Duration timeout = const Duration(seconds: 60),
    MavlinkCancellationToken? cancel,
  }) async {
    cancel?.throwIfCancelled();

    for (final node in onlineNodes) {
      if (excludeSystemIds == null || !excludeSystemIds.contains(node.systemId)) {
        return node;
      }
    }

    final completer = Completer<MavlinkNode>();
    late final StreamSubscription<MavlinkNode> subscription;

    subscription = onConnected.listen((node) {
      if (excludeSystemIds != null && excludeSystemIds.contains(node.systemId)) {
        return;
      }
      if (!completer.isCompleted) {
        completer.complete(node);
      }
    });

    StreamSubscription<void>? cancelSub;
    if (cancel != null) {
      if (cancel.isCancelled) {
        await subscription.cancel();
        throw MavlinkCancelledException();
      }
      cancelSub = cancel.onCancel.listen((_) {
        if (!completer.isCompleted) {
          completer.completeError(MavlinkCancelledException());
        }
      });
    }

    try {
      return await completer.future.timeout(timeout, onTimeout: () {
        throw MavlinkTimeoutException('Timed out waiting for vehicle heartbeat', timeout);
      });
    } finally {
      await subscription.cancel();
      await cancelSub?.cancel();
    }
  }

  void _onFrame(MavlinkFrame frame) {
    if (frame.message is! Heartbeat) {
      return;
    }

    final node = MavlinkNode(frame.systemId, frame.componentId);
    if (!_shouldWatch(node)) {
      return;
    }

    final heartbeat = frame.message as Heartbeat;
    final wasOnline = _online[node] ?? false;
    final now = DateTime.now();
    final tracked = TrackedHeartbeat(node: node, heartbeat: heartbeat, receivedAt: now, online: true);

    _states[node] = tracked;
    _online[node] = true;
    _heartbeatController.add(tracked);

    if (!wasOnline) {
      _connectedController.add(node);
    }
  }

  void _checkTimeouts() {
    final now = DateTime.now();
    for (final entry in List<MavlinkNode>.from(_states.keys)) {
      final state = _states[entry];
      if (state == null) {
        continue;
      }

      final timedOut = now.difference(state.receivedAt) > timeout;
      final wasOnline = _online[entry] ?? false;

      if (timedOut && wasOnline) {
        _online[entry] = false;
        _disconnectedController.add(entry);
        _heartbeatController.add(
          TrackedHeartbeat(node: entry, heartbeat: state.heartbeat, receivedAt: state.receivedAt, online: false),
        );
      }
    }
  }

  bool _shouldWatch(MavlinkNode node) {
    if (watch != null) {
      return watch!.contains(node);
    }
    if (watchSystemId != null) {
      return node.systemId == watchSystemId;
    }
    return true;
  }
}

/// Periodically sends HEARTBEAT on a [MavlinkSession].
class HeartbeatPublisher {
  HeartbeatPublisher({required this.session, required Heartbeat heartbeat, this.interval = const Duration(seconds: 1)})
      : _heartbeat = heartbeat;

  final MavlinkSession session;
  final Duration interval;

  Heartbeat _heartbeat;
  Timer? _timer;
  var _running = false;

  /// Payload sent on each heartbeat. Update fields via [updateHeartbeat].
  Heartbeat get heartbeat => _heartbeat;

  /// Replace the heartbeat payload (e.g. change [MavState]).
  void updateHeartbeat(Heartbeat heartbeat) {
    _heartbeat = heartbeat;
  }

  /// Apply [transform] to the current heartbeat payload.
  void mutateHeartbeat(Heartbeat Function(Heartbeat current) transform) {
    _heartbeat = transform(_heartbeat);
  }

  /// Start periodic transmission.
  void start() {
    if (_running) {
      return;
    }
    _running = true;
    unawaited(sendOnce());
    _timer = Timer.periodic(interval, (_) => unawaited(sendOnce()));
  }

  /// Stop periodic transmission.
  void stop() {
    _running = false;
    _timer?.cancel();
    _timer = null;
  }

  /// Send one heartbeat immediately.
  Future<void> sendOnce() async {
    try {
      await session.send(_heartbeat);
    } catch (e) {
      print('Heartbeat send failed: $e');
    }
  }
}

/// Convenience factories for common HEARTBEAT payloads.
class HeartbeatTemplates {
  HeartbeatTemplates._();

  /// Ground control station heartbeat.
  static Heartbeat gcs({required int mavlinkVersion}) {
    return Heartbeat(
      customMode: 0,
      type: MavType.mavTypeGcs,
      autopilot: MavAutopilot.mavAutopilotInvalid,
      baseMode: 0,
      systemStatus: MavState.mavStateActive,
      mavlinkVersion: mavlinkVersion,
    );
  }

  /// Generic onboard autopilot heartbeat.
  static Heartbeat autopilot({
    required int mavlinkVersion,
    MavType type = MavType.mavTypeQuadrotor,
    MavAutopilot autopilot = MavAutopilot.mavAutopilotPx4,
    MavState systemStatus = MavState.mavStateActive,
    uint32_t customMode = 0,
    uint8_t baseMode = 0,
  }) {
    return Heartbeat(
      customMode: customMode,
      type: type,
      autopilot: autopilot,
      baseMode: baseMode,
      systemStatus: systemStatus,
      mavlinkVersion: mavlinkVersion,
    );
  }

  /// Companion computer / onboard API heartbeat.
  static Heartbeat onboardApi({required int mavlinkVersion}) {
    return Heartbeat(
      customMode: 0,
      type: MavType.mavTypeOnboardController,
      autopilot: MavAutopilot.mavAutopilotInvalid,
      baseMode: 0,
      systemStatus: MavState.mavStateActive,
      mavlinkVersion: mavlinkVersion,
    );
  }
}
