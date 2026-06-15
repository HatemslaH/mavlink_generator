// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual telemetry request for the `rt_rc` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
void main() {
  final dialect = MavlinkDialectRt_rc();

  // Stream ATTITUDE (msg id 30) at 10 Hz (100_000 microseconds).
  final setInterval = CommandLong(
    param1: Attitude.msgId.toDouble(),
    param2: 100000,
    param3: 0,
    param4: 0,
    param5: 0,
    param6: 0,
    param7: 0,
    command: MavCmd.mavCmdSetMessageInterval,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    confirmation: 0,
  );
  final intervalFrame = frameFromGcs(setInterval, sequence: 1);
  logFrame('GCS ->', intervalFrame);
  final parsedInterval = roundTripMessage(dialect, setInterval);
  if (parsedInterval is CommandLong) {
    print(
      '  SET_MESSAGE_INTERVAL msgId=${parsedInterval.param1.toInt()} '
      'interval_us=${parsedInterval.param2.toInt()}',
    );
  }

  // One-shot ATTITUDE sample via MAV_CMD_REQUEST_MESSAGE.
  final requestOnce = CommandLong(
    param1: Attitude.msgId.toDouble(),
    param2: 0,
    param3: 0,
    param4: 0,
    param5: 0,
    param6: 0,
    param7: 0,
    command: MavCmd.mavCmdRequestMessage,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    confirmation: 0,
  );
  final onceFrame = frameFromGcs(requestOnce, sequence: 2);
  logFrame('GCS ->', onceFrame);
  roundTripMessage(dialect, requestOnce);

  // Simulated vehicle response: ATTITUDE telemetry frame.
  final attitude = Attitude(
    timeBootMs: 12345,
    roll: 0.01,
    pitch: -0.02,
    yaw: 1.57,
    rollspeed: 0,
    pitchspeed: 0,
    yawspeed: 0,
  );
  final telemetryFrame = frameFromDrone(attitude, sequence: 3);
  logFrame('Drone ->', telemetryFrame);
  final parsedAttitude = roundTripMessage(dialect, attitude);
  if (parsedAttitude is Attitude) {
    print(
      '  ATTITUDE roll=${parsedAttitude.roll} '
      'pitch=${parsedAttitude.pitch} yaw=${parsedAttitude.yaw}',
    );
  }
}
