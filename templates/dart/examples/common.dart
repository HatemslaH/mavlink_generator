export '../mavlink.dart';
import '../mavlink.dart';

/// Ground control station identity (MAVLink convention).
const gcsSystemId = 255;
const gcsComponentId = 190;

/// Simulated autopilot identity.
const droneSystemId = 1;
const droneComponentId = 1;

MavlinkFrame frameFromGcs(MavlinkMessage message, {int sequence = 0}) {
  return MavlinkFrame.v2(sequence, gcsSystemId, gcsComponentId, message);
}

MavlinkFrame frameFromDrone(MavlinkMessage message, {int sequence = 0}) {
  return MavlinkFrame.v2(sequence, droneSystemId, droneComponentId, message);
}

/// Encode a MAVLink `char[16]` parameter id from a short ASCII name.
List<char> paramIdFromString(String name) {
  final id = <char>[];
  for (final unit in name.codeUnits.take(16)) {
    id.add(unit);
  }
  while (id.length < 16) {
    id.add(0);
  }
  return id;
}

String paramIdToString(List<char> id) {
  final end = id.indexWhere((c) => c == 0);
  final slice = end == -1 ? id : id.sublist(0, end);
  return String.fromCharCodes(slice);
}

void logFrame(String direction, MavlinkFrame frame) {
  print(
    '$direction msgId=${frame.message.mavlinkMessageId} '
    'sys=${frame.systemId} comp=${frame.componentId}',
  );
}

/// Decode a message from its serialized payload (virtual round-trip).
MavlinkMessage? roundTripMessage(MavlinkDialect dialect, MavlinkMessage message) {
  return dialect.parse(message.mavlinkMessageId, message.serialize());
}
