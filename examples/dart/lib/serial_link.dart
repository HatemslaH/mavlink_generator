import 'dart:async';
import 'dart:typed_data';

import 'package:mavlink/protocols/mavlink_link.dart';
import 'package:serial_port_win32/serial_port_win32.dart';
import 'package:win32/win32.dart';

/// [MavlinkLink] implementation over a Windows COM/serial port (Win32 API).
class SerialMavlinkLink implements MavlinkLink {
  SerialMavlinkLink._(this._port);

  final SerialPort _port;
  final _receiveController = StreamController<Uint8List>.broadcast();
  var _closed = false;
  var _readLoopRunning = false;

  /// Open [portName] at [baudRate] (MAVLink SITL commonly uses 57600 or 115200).
  static SerialMavlinkLink open(String portName, {int baudRate = 57600}) {
    final port = SerialPort(portName, openNow: false);
    port.openWithSettings(
      BaudRate: baudRate,
      ByteSize: 8,
      Parity: NOPARITY,
      StopBits: ONESTOPBIT,
    );
    port.setFlowControlSignal(SerialPort.SETDTR);
    port.setFlowControlSignal(SerialPort.SETRTS);

    final link = SerialMavlinkLink._(port);
    link._startReading();
    return link;
  }

  void _startReading() {
    if (_readLoopRunning) {
      return;
    }
    _readLoopRunning = true;
    unawaited(_readLoop());
  }

  Future<void> _readLoop() async {
    while (!_closed && _port.isOpened) {
      try {
        final data = await _port.readBytes(
          4096,
          timeout: const Duration(milliseconds: 50),
        );
        if (_closed) {
          break;
        }
        if (data.isNotEmpty) {
          _receiveController.add(data);
        }
      } on Object catch (error, stackTrace) {
        if (!_closed) {
          _receiveController.addError(error, stackTrace);
        }
        break;
      }
    }
    _readLoopRunning = false;
  }

  @override
  Stream<Uint8List> get receive => _receiveController.stream;

  @override
  Future<void> send(Uint8List data) async {
    if (_closed) {
      throw StateError('SerialMavlinkLink is closed');
    }
    final ok = await _port.writeBytesFromUint8List(data);
    if (!ok) {
      throw StateError('Serial write failed on ${_port.toString()}');
    }
  }

  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    _port.close();
    await _receiveController.close();
  }
}
