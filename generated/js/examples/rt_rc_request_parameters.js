#!/usr/bin/env node
/** Virtual parameter service for the `rt_rc` dialect. */

import {
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  logFrame,
  paramIdFromString,
  paramIdToString,
  roundTripMessage,
  ParamRequestList,
  ParamRequestRead,
  ParamValue,
  MavParamType,
  MavlinkDialectRt_rc,
} from './common.js';

function main() {
  const dialect = new MavlinkDialectRt_rc();

  const listRequest = new ParamRequestList(droneSystemId, droneComponentId);
  logFrame('GCS ->', frameFromGcs(listRequest, 1));
  roundTripMessage(dialect, listRequest);

  const simulatedParams = [
    { id: 'SYSID_THISMAV', value: 1, index: 0 },
    { id: 'SYSID_MYGCS', value: 255, index: 1 },
    { id: 'COMPASS_ENABLE', value: 1, index: 2 },
  ];

  for (const param of simulatedParams) {
    const value = new ParamValue(
      param.value,
      simulatedParams.length,
      param.index,
      paramIdFromString(param.id),
      MavParamType.MAV_PARAM_TYPE_REAL32,
    );
    logFrame('Drone ->', frameFromDrone(value, param.index + 10));
    const parsed = roundTripMessage(dialect, value);
    if (parsed instanceof ParamValue) {
      console.log(
        `  PARAM_VALUE [${param.index + 1}/${simulatedParams.length}] ` +
          `${paramIdToString(parsed.param_id)}=${parsed.param_value}`,
      );
    }
  }

  const paramName = 'SYSID_THISMAV';
  const readRequest = new ParamRequestRead(
    -1,
    droneSystemId,
    droneComponentId,
    paramIdFromString(paramName),
  );
  logFrame('GCS ->', frameFromGcs(readRequest, 50));
  const parsedRead = roundTripMessage(dialect, readRequest);
  if (parsedRead instanceof ParamRequestRead) {
    console.log(`  PARAM_REQUEST_READ id=${paramIdToString(parsedRead.param_id)}`);
  }

  const singleValue = new ParamValue(
    1,
    simulatedParams.length,
    0,
    paramIdFromString(paramName),
    MavParamType.MAV_PARAM_TYPE_REAL32,
  );
  logFrame('Drone ->', frameFromDrone(singleValue, 51));
  roundTripMessage(dialect, singleValue);
}

main();
