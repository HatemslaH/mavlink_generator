import 'dart:async';
import 'dart:typed_data';

import '../mavlink_dialect.dart';
import '../mavlink_frame.dart';
import '../mavlink_message.dart';
import '../mavlink_parser.dart';
import '../mavlink_version.dart';
import 'mavlink_link.dart';

/// Thrown when an expected MAVLink message is not received in time.
class MavlinkTimeoutException implements Exception {
  MavlinkTimeoutException(this.message, this.timeout);

  final String message;
  final Duration timeout;

  @override
  String toString() => 'MavlinkTimeoutException: $message (timeout: $timeout)';
}

class _PendingFrameWait {
  _PendingFrameWait({
    required this.predicate,
    required this.completer,
    required this.timer,
  });

  final bool Function(MavlinkFrame frame) predicate;
  final Completer<MavlinkFrame> completer;
  final Timer timer;
}

/// Framing, sequencing, and message dispatch over a [MavlinkLink].
///
/// Protocol implementations use a session to send typed messages and wait for
/// responses without knowing whether the link is USB, UDP, or in-memory.
class MavlinkSession {
  MavlinkSession({
    required MavlinkDialect dialect,
    required MavlinkLink link,
    required this.systemId,
    required this.componentId,
    this.version = MavlinkVersion.v2,
  })  : _dialect = dialect,
        _link = link {
    _parser = MavlinkParser(_dialect);
    _parser.stream.listen(_onFrame);
    _subscription = _link.receive.listen(_parser.parse);
  }

  final MavlinkDialect _dialect;
  final MavlinkLink _link;
  final int systemId;
  final int componentId;
  final MavlinkVersion version;

  late final MavlinkParser _parser;
  late final StreamSubscription<Uint8List> _subscription;
  final _framesController = StreamController<MavlinkFrame>.broadcast();
  final List<_PendingFrameWait> _pendingWaits = [];
  final List<MavlinkFrame> _recentFrames = [];
  static const _recentFrameCapacity = 64;
  int _sequence = 0;
  var _closed = false;

  MavlinkDialect get dialect => _dialect;

  /// All frames parsed from the link (before filtering).
  Stream<MavlinkFrame> get frames => _framesController.stream;

  /// Send a typed MAVLink message as a framed packet.
  Future<void> send(MavlinkMessage message) async {
    if (_closed) {
      throw StateError('MavlinkSession is closed');
    }

    final frame = version == MavlinkVersion.v2
        ? MavlinkFrame.v2(_sequence++ & 0xff, systemId, componentId, message)
        : MavlinkFrame.v1(_sequence++ & 0xff, systemId, componentId, message);

    await _link.send(frame.serialize());
  }

  /// Wait for the first frame matching [predicate].
  Future<MavlinkFrame> waitForFrame({
    required bool Function(MavlinkFrame frame) predicate,
    Duration timeout = const Duration(seconds: 5),
  }) {
    final completer = Completer<MavlinkFrame>();
    late final _PendingFrameWait wait;
    wait = _PendingFrameWait(
      predicate: predicate,
      completer: completer,
      timer: Timer(timeout, () {
        _pendingWaits.remove(wait);
        if (!completer.isCompleted) {
          completer.completeError(
            MavlinkTimeoutException('Timed out waiting for frame', timeout),
          );
        }
      }),
    );
    _pendingWaits.add(wait);

    for (final frame in List<MavlinkFrame>.from(_recentFrames)) {
      if (!predicate(frame)) {
        continue;
      }
      _recentFrames.remove(frame);
      wait.timer.cancel();
      _pendingWaits.remove(wait);
      if (!completer.isCompleted) {
        completer.complete(frame);
      }
      return completer.future;
    }

    completer.future.whenComplete(() {
      wait.timer.cancel();
      _pendingWaits.remove(wait);
    });

    return completer.future;
  }

  /// Wait for the first message matching [predicate].
  Future<MavlinkMessage> waitForMessage({
    required bool Function(MavlinkMessage message) predicate,
    int? fromSystemId,
    int? fromComponentId,
    Duration timeout = const Duration(seconds: 5),
  }) async {
    final frame = await waitForFrame(
      predicate: (frame) {
        if (fromSystemId != null && frame.systemId != fromSystemId) {
          return false;
        }
        if (fromComponentId != null && frame.componentId != fromComponentId) {
          return false;
        }
        return predicate(frame.message);
      },
      timeout: timeout,
    );
    return frame.message;
  }

  /// Wait for the first message of type [T].
  Future<T> waitForMessageType<T extends MavlinkMessage>({
    int? fromSystemId,
    int? fromComponentId,
    Duration timeout = const Duration(seconds: 5),
  }) async {
    final message = await waitForMessage(
      predicate: (message) => message is T,
      fromSystemId: fromSystemId,
      fromComponentId: fromComponentId,
      timeout: timeout,
    );
    return message as T;
  }

  void _onFrame(MavlinkFrame frame) {
    if (_closed) {
      return;
    }

    _framesController.add(frame);
    _recentFrames.add(frame);
    if (_recentFrames.length > _recentFrameCapacity) {
      _recentFrames.removeAt(0);
    }

    for (final wait in List<_PendingFrameWait>.from(_pendingWaits)) {
      if (!wait.predicate(frame)) {
        continue;
      }

      wait.timer.cancel();
      _pendingWaits.remove(wait);
      _recentFrames.remove(frame);
      if (!wait.completer.isCompleted) {
        wait.completer.complete(frame);
      }
      break;
    }
  }

  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;

    for (final wait in List<_PendingFrameWait>.from(_pendingWaits)) {
      wait.timer.cancel();
      if (!wait.completer.isCompleted) {
        wait.completer.completeError(StateError('MavlinkSession is closed'));
      }
    }
    _pendingWaits.clear();

    await _subscription.cancel();
    await _framesController.close();
    await _link.close();
  }
}
