import 'dart:async';

/// Thrown when a MAVLink wait or long-running protocol operation is cancelled.
class MavlinkCancelledException implements Exception {
  MavlinkCancelledException([this.message = 'Operation cancelled']);

  final String message;

  @override
  String toString() => 'MavlinkCancelledException: $message';
}

/// Cooperative cancellation token for [MavlinkSession] waits and protocol flows.
///
/// Cancel with [cancel]; listeners and in-flight waits observe [isCancelled].
class MavlinkCancellationToken {
  MavlinkCancellationToken();

  final _controller = StreamController<void>.broadcast();
  var _cancelled = false;

  bool get isCancelled => _cancelled;

  /// Fires once when [cancel] is called.
  Stream<void> get onCancel => _controller.stream;

  void cancel() {
    if (_cancelled) {
      return;
    }
    _cancelled = true;
    _controller.add(null);
  }

  void throwIfCancelled() {
    if (_cancelled) {
      throw MavlinkCancelledException();
    }
  }

  void dispose() {
    if (!_controller.isClosed) {
      _controller.close();
    }
  }
}
