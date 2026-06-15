#!/usr/bin/env node
/** Virtual telemetry request for the `rt_rc` dialect. */

import {
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  logFrame,
  roundTripMessage,
  Attitude,
  CommandLong,
  MavCmd,
  MavlinkDialectRt_rc,
} from './common.js';

function main() {
  const dialect = new MavlinkDialectRt_rc();

  const setInterval = new CommandLong(
    Attitude.MSG_ID, 100000, 0, 0, 0, 0, 0,
    MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
    droneSystemId, droneComponentId, 0,
  );
  logFrame('GCS ->', frameFromGcs(setInterval, 1));
  const parsedInterval = roundTripMessage(dialect, setInterval);
  if (parsedInterval instanceof CommandLong) {
    console.log(
      `  SET_MESSAGE_INTERVAL msgId=${parsedInterval.param1} interval_us=${parsedInterval.param2}`,
    );
  }

  const requestOnce = new CommandLong(
    Attitude.MSG_ID, 0, 0, 0, 0, 0, 0,
    MavCmd.MAV_CMD_REQUEST_MESSAGE,
    droneSystemId, droneComponentId, 0,
  );
  logFrame('GCS ->', frameFromGcs(requestOnce, 2));
  roundTripMessage(dialect, requestOnce);

  const attitude = new Attitude(12345, 0.01, -0.02, 1.57, 0, 0, 0);
  logFrame('Drone ->', frameFromDrone(attitude, 3));
  const parsedAttitude = roundTripMessage(dialect, attitude);
  if (parsedAttitude instanceof Attitude) {
    console.log(
      `  ATTITUDE roll=${parsedAttitude.roll} pitch=${parsedAttitude.pitch} yaw=${parsedAttitude.yaw}`,
    );
  }
}

main();
