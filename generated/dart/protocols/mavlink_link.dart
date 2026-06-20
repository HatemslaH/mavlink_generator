import 'dart:async';
import 'dart:typed_data';

/// Transport-agnostic MAVLink byte stream.
///
/// Implement this interface for any physical or logical link (USB serial, UDP,
/// TCP, WebSocket, in-memory loopback, etc.). Protocol classes depend only on
/// [MavlinkLink], not on how bytes are moved.
abstract class MavlinkLink {
  /// Send raw MAVLink frame bytes to the remote peer.
  Future<void> send(Uint8List data);

  /// Incoming raw bytes from the remote peer.
  Stream<Uint8List> get receive;

  /// Release link resources. Default implementation is a no-op.
  Future<void> close() async {}
}

/// In-memory link for tests and virtual examples.
///
/// Connect two or more endpoints on the same [VirtualMavlinkBus]. Bytes sent by
/// one endpoint are delivered to every other endpoint on the bus.
class VirtualMavlinkBus {
  final List<_VirtualMavlinkEndpoint> _endpoints = [];

  /// Create a new endpoint on this bus.
  MavlinkLink createEndpoint() {
    final endpoint = _VirtualMavlinkEndpoint(this);
    _endpoints.add(endpoint);
    return endpoint;
  }

  void _deliver(Uint8List data, _VirtualMavlinkEndpoint sender) {
    for (final endpoint in _endpoints) {
      if (!identical(endpoint, sender)) {
        endpoint._emit(data);
      }
    }
  }

  /// Close every endpoint on the bus.
  Future<void> closeAll() async {
    final endpoints = List<_VirtualMavlinkEndpoint>.from(_endpoints);
    for (final endpoint in endpoints) {
      await endpoint.close();
    }
  }
}

class _VirtualMavlinkEndpoint implements MavlinkLink {
  _VirtualMavlinkEndpoint(this._bus);

  final VirtualMavlinkBus _bus;
  final _receiveController = StreamController<Uint8List>.broadcast();
  var _closed = false;

  @override
  Stream<Uint8List> get receive => _receiveController.stream;

  @override
  Future<void> send(Uint8List data) async {
    if (_closed) {
      throw StateError('VirtualMavlinkEndpoint is closed');
    }
    _bus._deliver(Uint8List.fromList(data), this);
  }

  void _emit(Uint8List data) {
    if (!_closed) {
      _receiveController.add(data);
    }
  }

  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    await _receiveController.close();
    _bus._endpoints.remove(this);
  }
}
