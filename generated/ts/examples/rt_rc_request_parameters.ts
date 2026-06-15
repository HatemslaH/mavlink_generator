/**
 * Virtual parameter service for the `rt_rc` dialect.
 *
 * Follows https://mavlink.io/en/services/parameter.html:
 * PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
 */

import {
  MavParamType,
  ParamRequestList,
  ParamRequestRead,
  ParamValue,
  MavlinkDialectRt_rc,
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  logFrame,
  paramIdFromString,
  paramIdToString,
  roundTripMessage,
} from './common';

interface SimulatedParam {
  id: string;
  value: number;
  index: number;
}

function main(): void {
  const dialect = new MavlinkDialectRt_rc();

  const listRequest = new ParamRequestList(droneSystemId, droneComponentId);
  const listFrame = frameFromGcs(listRequest, 1);
  logFrame('GCS ->', listFrame);
  roundTripMessage(dialect, listRequest);

  const simulatedParams: SimulatedParam[] = [
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
    const valueFrame = frameFromDrone(value, param.index + 10);
    logFrame('Drone ->', valueFrame);
    const parsed = roundTripMessage(dialect, value);
    if (parsed instanceof ParamValue) {
      console.log(
        `  PARAM_VALUE [${param.index + 1}/${simulatedParams.length}] ` +
          `${paramIdToString(parsed.paramId)}=${parsed.paramValue}`,
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
  const readFrame = frameFromGcs(readRequest, 50);
  logFrame('GCS ->', readFrame);
  const parsedRead = roundTripMessage(dialect, readRequest);
  if (parsedRead instanceof ParamRequestRead) {
    console.log(`  PARAM_REQUEST_READ id=${paramIdToString(parsedRead.paramId)}`);
  }

  const singleValue = new ParamValue(
    1,
    simulatedParams.length,
    0,
    paramIdFromString(paramName),
    MavParamType.MAV_PARAM_TYPE_REAL32,
  );
  const singleFrame = frameFromDrone(singleValue, 51);
  logFrame('Drone ->', singleFrame);
  roundTripMessage(dialect, singleValue);
}

main();
