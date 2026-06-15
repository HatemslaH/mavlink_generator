#!/usr/bin/env node
/** Example for the `rt_rc` dialect: serialize a Heartbeat frame and parse it back. */

import {
  frameFromGcs,
  logFrame,
  roundTripMessage,
  Heartbeat,
  MavType,
  MavAutopilot,
  MavState,
  MavlinkDialectRt_rc,
} from './common.js';

function main() {
  const dialect = new MavlinkDialectRt_rc();

  const heartbeat = new Heartbeat(
    0,
    MavType.MAV_TYPE_QUADROTOR,
    MavAutopilot.MAV_AUTOPILOT_PX4,
    0,
    MavState.MAV_STATE_ACTIVE,
    dialect.version,
  );

  const frame = frameFromGcs(heartbeat);
  const wire = frame.serialize();
  logFrame('GCS ->', frame);
  console.log(`Serialized HEARTBEAT (${wire.length} bytes)`);

  const parsed = roundTripMessage(dialect, heartbeat);
  if (parsed instanceof Heartbeat) {
    console.log(`Parsed HEARTBEAT type=${parsed.type} status=${parsed.system_status}`);
  }
}

main();
