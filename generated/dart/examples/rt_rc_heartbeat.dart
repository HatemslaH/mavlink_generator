// ignore_for_file: avoid_print

import 'common.dart';

/// Example for the `rt_rc` dialect: serialize a [Heartbeat] frame and
/// parse it back with [MavlinkDialectRt_rc].
void main() {
  final dialect = MavlinkDialectRt_rc();

  final heartbeat = Heartbeat(
    customMode: 0,
    type: MavType.mavTypeQuadrotor,
    autopilot: MavAutopilot.mavAutopilotPx4,
    baseMode: 0,
    systemStatus: MavState.mavStateActive,
    mavlinkVersion: dialect.version,
  );

  final frame = frameFromGcs(heartbeat);
  final bytes = frame.serialize();
  logFrame('GCS ->', frame);
  print('Serialized HEARTBEAT (${bytes.length} bytes)');

  final parsed = roundTripMessage(dialect, heartbeat);
  if (parsed is Heartbeat) {
    print('Parsed HEARTBEAT type=${parsed.type} status=${parsed.systemStatus}');
  }
}
