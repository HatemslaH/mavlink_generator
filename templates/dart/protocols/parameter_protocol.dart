import 'dart:async';

import '../mavlink.dart';
import 'mavlink_session.dart';
import 'param_codec.dart';

/// Decoded onboard parameter entry.
class ParamEntry {
  const ParamEntry({
    required this.id,
    required this.value,
    required this.type,
    required this.index,
    required this.count,
  });

  final String id;
  final num value;
  final MavParamType type;
  final int index;
  final int count;

  factory ParamEntry.fromParamValue(ParamValue message) {
    return ParamEntry(
      id: ParamCodec.paramIdToString(message.paramId),
      value: ParamCodec.decodeValue(message.paramValue, message.paramType),
      type: message.paramType,
      index: message.paramIndex,
      count: message.paramCount,
    );
  }
}

/// GCS-side MAVLink parameter protocol client.
///
/// Implements list/read/write flows from
/// https://mavlink.io/en/services/parameter.html
class ParameterProtocol {
  ParameterProtocol({
    required this.session,
    required this.targetSystem,
    required this.targetComponent,
    this.idleTimeout = const Duration(milliseconds: 500),
    this.requestTimeout = const Duration(seconds: 3),
  });

  final MavlinkSession session;
  final int targetSystem;
  final int targetComponent;
  final Duration idleTimeout;
  final Duration requestTimeout;

  /// Request and collect the full parameter set from the vehicle.
  Future<List<ParamEntry>> fetchAll() async {
    await session.send(ParamRequestList(targetSystem: targetSystem, targetComponent: targetComponent));

    final entries = <ParamEntry>[];
    var expectedCount = -1;
    final seenIndices = <int>{};

    while (true) {
      final value = await session.waitForMessage(
        predicate: (message) {
          if (message is! ParamValue) {
            return false;
          }
          return !seenIndices.contains(message.paramIndex);
        },
        fromSystemId: targetSystem,
        timeout: expectedCount == -1 ? requestTimeout : idleTimeout,
      );

      final paramValue = value as ParamValue;
      seenIndices.add(paramValue.paramIndex);

      if (expectedCount == -1) {
        expectedCount = paramValue.paramCount;
      }

      entries.add(ParamEntry.fromParamValue(paramValue));

      if (entries.length >= expectedCount) {
        break;
      }
    }

    return entries;
  }

  /// Read a single parameter by name (`paramIndex` = -1).
  Future<ParamEntry> readByName(String name) {
    return read(paramId: name);
  }

  /// Read a single parameter by onboard index.
  Future<ParamEntry> readByIndex(int index) {
    return read(paramIndex: index);
  }

  /// Read one parameter by id or index.
  Future<ParamEntry> read({String? paramId, int paramIndex = -1}) async {
    if (paramId == null && paramIndex < 0) {
      throw ArgumentError('Either paramId or a non-negative paramIndex is required');
    }

    await session.send(
      ParamRequestRead(
        paramIndex: paramIndex,
        targetSystem: targetSystem,
        targetComponent: targetComponent,
        paramId: ParamCodec.paramIdFromString(paramId ?? ''),
      ),
    );

    final value = await session.waitForMessageType<ParamValue>(fromSystemId: targetSystem, timeout: requestTimeout);

    return ParamEntry.fromParamValue(value);
  }

  /// Write a parameter and wait for the broadcast [ParamValue] acknowledgment.
  Future<ParamEntry> write({required String name, required num value, required MavParamType type}) async {
    await session.send(
      ParamSet(
        paramValue: ParamCodec.encodeValue(value, type),
        targetSystem: targetSystem,
        targetComponent: targetComponent,
        paramId: ParamCodec.paramIdFromString(name),
        paramType: type,
      ),
    );

    final ack = await session.waitForMessage(
      predicate: (message) {
        if (message is! ParamValue) {
          return false;
        }
        return ParamCodec.paramIdToString(message.paramId) == name;
      },
      fromSystemId: targetSystem,
      timeout: requestTimeout,
    );

    return ParamEntry.fromParamValue(ack as ParamValue);
  }
}

/// Vehicle-side parameter store handler for embedding in autopilot code.
class ParameterServer {
  ParameterServer({required this.session, Map<String, ({num value, MavParamType type})>? initialValues})
    : _values = Map<String, ({num value, MavParamType type})>.from(initialValues ?? {}) {
    _subscription = session.frames.listen(_onFrame);
  }

  final MavlinkSession session;
  final Map<String, ({num value, MavParamType type})> _values;
  late final StreamSubscription<MavlinkFrame> _subscription;

  Map<String, ({num value, MavParamType type})> get values => Map.unmodifiable(_values);

  Future<void> close() async {
    await _subscription.cancel();
  }

  void set(String name, num value, MavParamType type) {
    _values[name] = (value: value, type: type);
  }

  Future<void> _onFrame(MavlinkFrame frame) async {
    final message = frame.message;

    if (message is ParamRequestList) {
      if (message.targetSystem != session.systemId && message.targetSystem != MavComponent.mavCompIdAll) {
        return;
      }
      await _broadcastAll();
      return;
    }

    if (message is ParamRequestRead) {
      if (message.targetSystem != session.systemId && message.targetSystem != MavComponent.mavCompIdAll) {
        return;
      }
      final entry = _resolveRead(message);
      if (entry != null) {
        await _sendValue(entry.key, entry.value, _indexOf(entry.key));
      }
      return;
    }

    if (message is ParamSet) {
      if (message.targetSystem != session.systemId) {
        return;
      }
      final name = ParamCodec.paramIdToString(message.paramId);
      _values[name] = (value: ParamCodec.decodeValue(message.paramValue, message.paramType), type: message.paramType);
      await _sendValue(name, _values[name]!, _indexOf(name));
    }
  }

  Future<void> _broadcastAll() async {
    final names = _values.keys.toList();
    for (var index = 0; index < names.length; index++) {
      await _sendValue(names[index], _values[names[index]]!, index);
    }
  }

  Future<void> _sendValue(String name, ({num value, MavParamType type}) entry, int index) async {
    await session.send(
      ParamValue(
        paramValue: ParamCodec.encodeValue(entry.value, entry.type),
        paramCount: _values.length,
        paramIndex: index,
        paramId: ParamCodec.paramIdFromString(name),
        paramType: entry.type,
      ),
    );
  }

  MapEntry<String, ({num value, MavParamType type})>? _resolveRead(ParamRequestRead request) {
    if (request.paramIndex >= 0) {
      final names = _values.keys.toList();
      if (request.paramIndex >= names.length) {
        return null;
      }
      final name = names[request.paramIndex];
      return MapEntry(name, _values[name]!);
    }

    final name = ParamCodec.paramIdToString(request.paramId);
    final entry = _values[name];
    if (entry == null) {
      return null;
    }
    return MapEntry(name, entry);
  }

  int _indexOf(String name) {
    return _values.keys.toList().indexOf(name);
  }
}
