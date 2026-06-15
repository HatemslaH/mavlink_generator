/**
 * Virtual telemetry request for the `rt_rc` dialect.
 *
 * Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
 * MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
 */

import {
  Attitude,
  CommandLong,
  MavCmd,
  MavlinkDialectRt_rc,
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  logFrame,
  roundTripMessage,
} from './common';

function main(): void {
  const dialect = new MavlinkDialectRt_rc();

  const setInterval = new CommandLong(
    Attitude.MSG_ID,
    100000,
    0, 0, 0, 0, 0,
    MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
    droneSystemId,
    droneComponentId,
    0,
  );
  const intervalFrame = frameFromGcs(setInterval, 1);
  logFrame('GCS ->', intervalFrame);
  const parsedInterval = roundTripMessage(dialect, setInterval);
  if (parsedInterval instanceof CommandLong) {
    console.log(
      `  SET_MESSAGE_INTERVAL msgId=${parsedInterval.param1} ` +
        `interval_us=${parsedInterval.param2}`,
    );
  }

  const requestOnce = new CommandLong(
    Attitude.MSG_ID,
    0, 0, 0, 0, 0, 0,
    MavCmd.MAV_CMD_REQUEST_MESSAGE,
    droneSystemId,
    droneComponentId,
    0,
  );
  const onceFrame = frameFromGcs(requestOnce, 2);
  logFrame('GCS ->', onceFrame);
  roundTripMessage(dialect, requestOnce);

  const attitude = new Attitude(12345, 0.01, -0.02, 1.57, 0, 0, 0);
  const telemetryFrame = frameFromDrone(attitude, 3);
  logFrame('Drone ->', telemetryFrame);
  const parsedAttitude = roundTripMessage(dialect, attitude);
  if (parsedAttitude instanceof Attitude) {
    console.log(
      `  ATTITUDE roll=${parsedAttitude.roll} ` +
        `pitch=${parsedAttitude.pitch} yaw=${parsedAttitude.yaw}`,
    );
  }
}

main();
