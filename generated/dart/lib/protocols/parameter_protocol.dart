import 'dart:async';

import '../mavlink.dart';
import 'mavlink_cancellation.dart';
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

/// Progress callback for [ParameterProtocol.fetchAll] and [fetchAllStream].
typedef ParamProgressCallback = void Function(ParamEntry entry, int received, int expected);

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

  final Map<String, ParamEntry> _cache = {};

  /// Last fetched or written parameters keyed by name (unmodifiable view).
  Map<String, ParamEntry> get cache => Map.unmodifiable(_cache);

  void clearCache() => _cache.clear();

  void _remember(ParamEntry entry) => _cache[entry.id] = entry;

  MavParamType? typeForName(String name) => _cache[name]?.type;

  /// Request and collect the full parameter set from the vehicle.
  ///
  /// Optional [onProgress] is called after each [ParamValue] is decoded.
  /// Pass [cancel] to abort the stream early.
  Future<List<ParamEntry>> fetchAll({
    ParamProgressCallback? onProgress,
    MavlinkCancellationToken? cancel,
  }) async {
    final entries = <ParamEntry>[];
    await for (final entry in fetchAllStream(cancel: cancel)) {
      entries.add(entry);
      onProgress?.call(entry, entries.length, entry.count);
    }
    return entries;
  }

  /// Stream parameters as they arrive from the vehicle.
  Stream<ParamEntry> fetchAllStream({MavlinkCancellationToken? cancel}) async* {
    cancel?.throwIfCancelled();

    await session.send(ParamRequestList(targetSystem: targetSystem, targetComponent: targetComponent));

    var expectedCount = -1;
    final seenIndices = <int>{};

    while (true) {
      cancel?.throwIfCancelled();

      final value = await session.waitForMessage(
        predicate: (message) {
          if (message is! ParamValue) {
            return false;
          }
          return !seenIndices.contains(message.paramIndex);
        },
        fromSystemId: targetSystem,
        timeout: expectedCount == -1 ? requestTimeout : idleTimeout,
        cancel: cancel,
      );

      final paramValue = value as ParamValue;
      seenIndices.add(paramValue.paramIndex);

      if (expectedCount == -1) {
        expectedCount = paramValue.paramCount;
      }

      final entry = ParamEntry.fromParamValue(paramValue);
      _remember(entry);
      yield entry;

      if (seenIndices.length >= expectedCount) {
        break;
      }
    }
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
  Future<ParamEntry> read({String? paramId, int paramIndex = -1, MavlinkCancellationToken? cancel}) async {
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

    final value = await session.waitForMessageType<ParamValue>(
      fromSystemId: targetSystem,
      timeout: requestTimeout,
      cancel: cancel,
    );

    final entry = ParamEntry.fromParamValue(value);
    _remember(entry);
    return entry;
  }

  /// Write a parameter and wait for the broadcast [ParamValue] acknowledgment.
  Future<ParamEntry> write({
    required String name,
    required num value,
    required MavParamType type,
    MavlinkCancellationToken? cancel,
  }) async {
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
      cancel: cancel,
    );

    final entry = ParamEntry.fromParamValue(ack as ParamValue);
    _remember(entry);
    return entry;
  }

  /// Write using [type] when provided, otherwise the cached type for [name].
  ///
  /// Falls back to [MavParamType.mavParamTypeReal32] when the type is unknown.
  Future<ParamEntry> writeByName(String name, num value, {MavParamType? type, MavlinkCancellationToken? cancel}) {
    final resolvedType = type ?? typeForName(name) ?? MavParamType.mavParamTypeReal32;
    return write(name: name, value: value, type: resolvedType, cancel: cancel);
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
