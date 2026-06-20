#!/usr/bin/env node
/** Parameter protocol example for the `rt_rc` dialect. */

import {
  MavParamType,
  ParameterProtocol,
  ParameterServer,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  MavlinkDialectRt_rc,
} from './protocols_common.js';

async function main() {
  const dialect = new MavlinkDialectRt_rc();
  const link = createVirtualLink(dialect);

  const parameterServer = new ParameterServer({
    session: link.drone,
    initialValues: {
      SYSID_THISMAV: { value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 },
      SYSID_MYGCS: { value: 255, type: MavParamType.MAV_PARAM_TYPE_INT32 },
      COMPASS_ENABLE: { value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 },
    },
  });

  const parameterProtocol = new ParameterProtocol({
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  });

  const allParams = await parameterProtocol.fetchAll({
    onProgress: (entry, received, expected) => {
      console.log(`  [${received}/${expected}] ${entry.id}=${entry.value}`);
    },
  });
  console.log(
    `Fetched ${allParams.length} parameters (cache size=${Object.keys(parameterProtocol.cache).length})`,
  );

  const single = await parameterProtocol.readByName('SYSID_THISMAV');
  console.log(`Read SYSID_THISMAV=${single.value}`);

  const updated = await parameterProtocol.writeByName('COMPASS_ENABLE', 0);
  console.log(`Wrote COMPASS_ENABLE=${updated.value} (${updated.type})`);

  await parameterServer.close();
  await closeVirtualLink({ bus: link.bus, gcs: link.gcs, drone: link.drone });
}

main();
