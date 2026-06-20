use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct JavaScriptExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/js/examples/README.md"),
    ),
    (
        "common.js",
        include_str!("../../../templates/js/examples/common.js"),
    ),
    (
        "protocols_common.js",
        include_str!("../../../templates/js/examples/protocols_common.js"),
    ),
];

const LOW_LEVEL_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

const PROTOCOL_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("protocol_mission", render_protocol_mission_example),
    ("protocol_parameters", render_protocol_parameters_example),
    ("protocol_command", render_protocol_command_example),
    ("protocol_heartbeat", render_protocol_heartbeat_example),
    ("protocol_vehicle", render_protocol_vehicle_example),
    ("protocol_subscribe", render_protocol_subscribe_example),
];

impl LanguageExampleGenerator for JavaScriptExampleGenerator {
    fn static_files(&self) -> Vec<ExampleFile> {
        STATIC_TEMPLATES
            .iter()
            .map(|(name, content)| ExampleFile {
                relative_path: PathBuf::from(*name),
                content: (*content).to_string(),
            })
            .collect()
    }

    fn generated_files(&self, dialect_stems: &[String]) -> Vec<ExampleFile> {
        dialect_stems
            .iter()
            .flat_map(|stem| {
                let stem = stem.clone();
                LOW_LEVEL_EXAMPLES
                    .iter()
                    .chain(PROTOCOL_EXAMPLES.iter())
                    .map(move |(suffix, render)| ExampleFile {
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.js")),
                        content: render(&stem),
                    })
            })
            .collect()
    }
}

fn dialect_class_name(stem: &str) -> String {
    format!("MavlinkDialect{}", capitalize(stem))
}

fn render_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Example for the `{dialect_stem}` dialect: serialize a Heartbeat frame and parse it back. */

import {{
  frameFromGcs,
  logFrame,
  roundTripMessage,
  Heartbeat,
  MavType,
  MavAutopilot,
  MavState,
  {dialect_class},
}} from './common.js';

function main() {{
  const dialect = new {dialect_class}();

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
  console.log(`Serialized HEARTBEAT (${{wire.length}} bytes)`);

  const parsed = roundTripMessage(dialect, heartbeat);
  if (parsed instanceof Heartbeat) {{
    console.log(`Parsed HEARTBEAT type=${{parsed.type}} status=${{parsed.system_status}}`);
  }}
}}

main();
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Virtual mission upload for the `{dialect_stem}` dialect. */

import {{
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  gcsComponentId,
  gcsSystemId,
  logFrame,
  roundTripMessage,
  MissionAck,
  MissionCount,
  MissionItem,
  MissionRequest,
  MavCmd,
  MavFrame,
  MavMissionResult,
  MavMissionType,
  {dialect_class},
}} from './common.js';

function main() {{
  const dialect = new {dialect_class}();
  const missionType = MavMissionType.MAV_MISSION_TYPE_MISSION;

  const missionItems = [
    new MissionItem(
      0, 2, 0, 0,
      47.397742, 8.545594, 50,
      0,
      MavCmd.MAV_CMD_NAV_WAYPOINT,
      droneSystemId, droneComponentId,
      MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1, missionType,
    ),
    new MissionItem(
      0, 2, 0, 0,
      47.398000, 8.546000, 50,
      1,
      MavCmd.MAV_CMD_NAV_WAYPOINT,
      droneSystemId, droneComponentId,
      MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
      0, 1, missionType,
    ),
  ];

  let seq = 0;

  const count = new MissionCount(
    missionItems.length,
    droneSystemId,
    droneComponentId,
    missionType,
  );
  logFrame('GCS ->', frameFromGcs(count, 1));
  roundTripMessage(dialect, count);

  while (seq < missionItems.length) {{
    const request = new MissionRequest(
      seq,
      gcsSystemId,
      gcsComponentId,
      missionType,
    );
    logFrame('Drone ->', frameFromDrone(request, seq + 10));
    roundTripMessage(dialect, request);

    const item = missionItems[seq];
    logFrame('GCS ->', frameFromGcs(item, seq + 20));
    const parsedItem = roundTripMessage(dialect, item);
    if (parsedItem instanceof MissionItem) {{
      console.log(`  uploaded seq=${{parsedItem.seq}} cmd=${{parsedItem.command}}`);
    }}

    seq += 1;
  }}

  const ack = new MissionAck(
    gcsSystemId,
    gcsComponentId,
    MavMissionResult.MAV_MISSION_ACCEPTED,
    missionType,
  );
  logFrame('Drone ->', frameFromDrone(ack, 99));
  const parsedAck = roundTripMessage(dialect, ack);
  if (parsedAck instanceof MissionAck) {{
    console.log(`Mission upload complete: ${{parsedAck.type}}`);
  }}
}}

main();
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Virtual telemetry request for the `{dialect_stem}` dialect. */

import {{
  droneComponentId,
  droneSystemId,
  frameFromDrone,
  frameFromGcs,
  logFrame,
  roundTripMessage,
  Attitude,
  CommandLong,
  MavCmd,
  {dialect_class},
}} from './common.js';

function main() {{
  const dialect = new {dialect_class}();

  const setInterval = new CommandLong(
    Attitude.MSG_ID, 100000, 0, 0, 0, 0, 0,
    MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
    droneSystemId, droneComponentId, 0,
  );
  logFrame('GCS ->', frameFromGcs(setInterval, 1));
  const parsedInterval = roundTripMessage(dialect, setInterval);
  if (parsedInterval instanceof CommandLong) {{
    console.log(
      `  SET_MESSAGE_INTERVAL msgId=${{parsedInterval.param1}} interval_us=${{parsedInterval.param2}}`,
    );
  }}

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
  if (parsedAttitude instanceof Attitude) {{
    console.log(
      `  ATTITUDE roll=${{parsedAttitude.roll}} pitch=${{parsedAttitude.pitch}} yaw=${{parsedAttitude.yaw}}`,
    );
  }}
}}

main();
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Virtual parameter service for the `{dialect_stem}` dialect. */

import {{
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
  {dialect_class},
}} from './common.js';

function main() {{
  const dialect = new {dialect_class}();

  const listRequest = new ParamRequestList(droneSystemId, droneComponentId);
  logFrame('GCS ->', frameFromGcs(listRequest, 1));
  roundTripMessage(dialect, listRequest);

  const simulatedParams = [
    {{ id: 'SYSID_THISMAV', value: 1, index: 0 }},
    {{ id: 'SYSID_MYGCS', value: 255, index: 1 }},
    {{ id: 'COMPASS_ENABLE', value: 1, index: 2 }},
  ];

  for (const param of simulatedParams) {{
    const value = new ParamValue(
      param.value,
      simulatedParams.length,
      param.index,
      paramIdFromString(param.id),
      MavParamType.MAV_PARAM_TYPE_REAL32,
    );
    logFrame('Drone ->', frameFromDrone(value, param.index + 10));
    const parsed = roundTripMessage(dialect, value);
    if (parsed instanceof ParamValue) {{
      console.log(
        `  PARAM_VALUE [${{param.index + 1}}/${{simulatedParams.length}}] ` +
          `${{paramIdToString(parsed.param_id)}}=${{parsed.param_value}}`,
      );
    }}
  }}

  const paramName = 'SYSID_THISMAV';
  const readRequest = new ParamRequestRead(
    -1,
    droneSystemId,
    droneComponentId,
    paramIdFromString(paramName),
  );
  logFrame('GCS ->', frameFromGcs(readRequest, 50));
  const parsedRead = roundTripMessage(dialect, readRequest);
  if (parsedRead instanceof ParamRequestRead) {{
    console.log(`  PARAM_REQUEST_READ id=${{paramIdToString(parsedRead.param_id)}}`);
  }}

  const singleValue = new ParamValue(
    1,
    simulatedParams.length,
    0,
    paramIdFromString(paramName),
    MavParamType.MAV_PARAM_TYPE_REAL32,
  );
  logFrame('Drone ->', frameFromDrone(singleValue, 51));
  roundTripMessage(dialect, singleValue);
}}

main();
"#
    )
}

fn render_protocol_mission_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Mission protocol example for the `{dialect_stem}` dialect over VirtualMavlinkBus. */

import {{
  CommandProtocol,
  CommandServer,
  MissionItems,
  MissionProtocol,
  MissionServer,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  {dialect_class},
}} from './protocols_common.js';

async function main() {{
  const dialect = new {dialect_class}();
  const link = createVirtualLink(dialect);

  const missionServer = new MissionServer({{ session: link.drone }});
  const commandServer = new CommandServer({{ session: link.drone }});
  const missionProtocol = new MissionProtocol({{
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  }});

  const plan = [
    MissionItems.waypoint({{
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    }}),
    MissionItems.waypoint({{
      seq: 1,
      latitude: 47.398,
      longitude: 8.546,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    }}),
  ];

  const uploadResult = await missionProtocol.upload(plan, {{
    onProgress: (sent, total, item) => {{
      console.log(`Upload progress ${{sent}}/${{total}} seq=${{item.seq}} cmd=${{item.command}}`);
    }},
  }});
  console.log(`Mission upload result: ${{uploadResult}}`);
  console.log(`Vehicle stored ${{missionServer.items.length}} items`);

  const downloaded = await missionProtocol.download({{
    onProgress: (received, total, item) => {{
      console.log(`Download progress ${{received}}/${{total}} seq=${{item.seq}}`);
    }},
  }});
  console.log(`Downloaded ${{downloaded.length}} mission items`);

  const commandProtocol = new CommandProtocol({{
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  }});
  const setCurrent = await missionProtocol.setCurrentWithCommand(0, {{
    command: commandProtocol,
  }});
  console.log(`Set current seq=${{setCurrent.sequence}} ack=${{setCurrent.commandAck?.result}}`);

  const clearResult = await missionProtocol.clear();
  console.log(`Mission clear result: ${{clearResult}}`);

  await missionServer.close();
  await commandServer.close();
  await closeVirtualLink({{ bus: link.bus, gcs: link.gcs, drone: link.drone }});
}}

main();
"#
    )
}

fn render_protocol_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Parameter protocol example for the `{dialect_stem}` dialect. */

import {{
  MavParamType,
  ParameterProtocol,
  ParameterServer,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  {dialect_class},
}} from './protocols_common.js';

async function main() {{
  const dialect = new {dialect_class}();
  const link = createVirtualLink(dialect);

  const parameterServer = new ParameterServer({{
    session: link.drone,
    initialValues: {{
      SYSID_THISMAV: {{ value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 }},
      SYSID_MYGCS: {{ value: 255, type: MavParamType.MAV_PARAM_TYPE_INT32 }},
      COMPASS_ENABLE: {{ value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 }},
    }},
  }});

  const parameterProtocol = new ParameterProtocol({{
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  }});

  const allParams = await parameterProtocol.fetchAll({{
    onProgress: (entry, received, expected) => {{
      console.log(`  [${{received}}/${{expected}}] ${{entry.id}}=${{entry.value}}`);
    }},
  }});
  console.log(
    `Fetched ${{allParams.length}} parameters (cache size=${{Object.keys(parameterProtocol.cache).length}})`,
  );

  const single = await parameterProtocol.readByName('SYSID_THISMAV');
  console.log(`Read SYSID_THISMAV=${{single.value}}`);

  const updated = await parameterProtocol.writeByName('COMPASS_ENABLE', 0);
  console.log(`Wrote COMPASS_ENABLE=${{updated.value}} (${{updated.type}})`);

  await parameterServer.close();
  await closeVirtualLink({{ bus: link.bus, gcs: link.gcs, drone: link.drone }});
}}

main();
"#
    )
}

fn render_protocol_command_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Command protocol example for the `{dialect_stem}` dialect. */

import {{
  Attitude,
  CommandProtocol,
  CommandServer,
  MavResult,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  {dialect_class},
}} from './protocols_common.js';

async function main() {{
  const dialect = new {dialect_class}();
  const link = createVirtualLink(dialect);

  const commandServer = new CommandServer({{
    session: link.drone,
    onCommandLong: async (command) => {{
      console.log(
        `Vehicle received COMMAND_LONG: ${{command.command}} ` +
          `p1=${{command.param1}} p2=${{command.param2}}`,
      );
      return MavResult.MAV_RESULT_ACCEPTED;
    }},
  }});

  const commandProtocol = new CommandProtocol({{
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  }});

  const intervalAck = await commandProtocol.setMessageInterval(Attitude.MSG_ID, 100000);
  console.log(`SET_MESSAGE_INTERVAL ack: ${{intervalAck.result}}`);

  const requestAck = await commandProtocol.requestMessage(Attitude.MSG_ID);
  console.log(`REQUEST_MESSAGE ack: ${{requestAck.result}}`);

  const armAck = await commandProtocol.arm();
  console.log(`ARM ack: ${{armAck.result}}`);

  const disarmAck = await commandProtocol.disarm();
  console.log(`DISARM ack: ${{disarmAck.result}}`);

  await commandServer.close();
  await closeVirtualLink({{ bus: link.bus, gcs: link.gcs, drone: link.drone }});
}}

main();
"#
    )
}

fn render_protocol_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Heartbeat protocol example for the `{dialect_stem}` dialect. */

import {{
  HeartbeatMonitor,
  HeartbeatPublisher,
  HeartbeatTemplates,
  closeVirtualLink,
  createVirtualLink,
  gcsSystemId,
  {dialect_class},
}} from './protocols_common.js';

function delay(ms) {{
  return new Promise((resolve) => setTimeout(resolve, ms));
}}

async function main() {{
  const dialect = new {dialect_class}();
  const link = createVirtualLink(dialect);

  const gcsPublisher = new HeartbeatPublisher({{
    session: link.gcs,
    heartbeat: HeartbeatTemplates.gcs({{ mavlinkVersion: dialect.version }}),
    intervalMs: 500,
  }});

  const dronePublisher = new HeartbeatPublisher({{
    session: link.drone,
    heartbeat: HeartbeatTemplates.autopilot({{ mavlinkVersion: dialect.version }}),
    intervalMs: 500,
  }});

  const gcsMonitor = new HeartbeatMonitor({{
    session: link.gcs,
    timeoutMs: 2000,
  }});

  gcsMonitor.start();
  gcsPublisher.start();
  dronePublisher.start();

  const vehicle = await gcsMonitor.waitForVehicle({{
    excludeSystemIds: new Set([gcsSystemId]),
    timeoutMs: 5000,
  }});
  console.log(`Vehicle discovered: ${{vehicle}}`);
  console.log(`Drone online: ${{gcsMonitor.isOnline(vehicle)}}`);
  const state = gcsMonitor.stateFor(vehicle);
  if (state != null) {{
    console.log(
      `Drone heartbeat: type=${{state.heartbeat.type}} status=${{state.heartbeat.system_status}}`,
    );
  }}

  dronePublisher.stop();
  await delay(2500);
  console.log(`Drone online after stop: ${{gcsMonitor.isOnline(vehicle)}}`);

  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink({{ bus: link.bus, gcs: link.gcs, drone: link.drone }});
}}

main();
"#
    )
}

fn render_protocol_vehicle_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** MavlinkGcs / MavlinkVehicleClient facade example for `{dialect_stem}`. */

import {{
  CommandServer,
  Heartbeat,
  HeartbeatPublisher,
  HeartbeatTemplates,
  MavParamType,
  MavlinkGcs,
  MavlinkSession,
  ParameterServer,
  VirtualMavlinkBus,
  droneComponentId,
  droneSystemId,
  gcsComponentId,
  gcsSystemId,
  {dialect_class},
}} from './protocols_common.js';

async function main() {{
  const dialect = new {dialect_class}();
  const bus = new VirtualMavlinkBus();
  const gcsLink = bus.createEndpoint();
  const droneLink = bus.createEndpoint();

  const gcs = MavlinkGcs.connect({{
    dialect,
    link: gcsLink,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  }});

  const droneSession = new MavlinkSession({{
    dialect,
    link: droneLink,
    systemId: droneSystemId,
    componentId: droneComponentId,
  }});

  const dronePublisher = new HeartbeatPublisher({{
    session: droneSession,
    heartbeat: HeartbeatTemplates.autopilot({{ mavlinkVersion: dialect.version }}),
    intervalMs: 500,
  }});

  const parameterServer = new ParameterServer({{
    session: droneSession,
    initialValues: {{
      SYSID_THISMAV: {{ value: 1, type: MavParamType.MAV_PARAM_TYPE_INT32 }},
    }},
  }});

  const commandServer = new CommandServer({{ session: droneSession }});

  gcs.start();
  dronePublisher.start();

  const client = await gcs.waitForVehicle({{ excludeSystemIds: new Set([gcsSystemId]) }});
  console.log(`Connected to vehicle ${{client.vehicle}}`);

  const params = await client.parameters.fetchAll();
  console.log(`Vehicle has ${{params.length}} parameters`);

  const ack = await client.command.requestMessage(Heartbeat.MSG_ID);
  console.log(`REQUEST_MESSAGE ack: ${{ack.result}}`);

  await parameterServer.close();
  await commandServer.close();
  dronePublisher.stop();
  await droneSession.close();
  await gcs.close();
  await bus.closeAll();
}}

main();
"#
    )
}

fn render_protocol_subscribe_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"#!/usr/bin/env node
/** Typed message subscription example for the `{dialect_stem}` dialect. */

import {{
  Attitude,
  MavlinkNode,
  closeVirtualLink,
  createVirtualLink,
  droneComponentId,
  droneSystemId,
  {dialect_class},
}} from './protocols_common.js';

function delay(ms) {{
  return new Promise((resolve) => setTimeout(resolve, ms));
}}

async function main() {{
  const dialect = new {dialect_class}();
  const link = createVirtualLink(dialect);
  const vehicle = new MavlinkNode(droneSystemId, droneComponentId);

  const attitudeSamples = [];
  const subscription = link.gcs.listenMessage(
    Attitude,
    (message) => attitudeSamples.push(message),
    {{ fromSystemId: vehicle.systemId }},
  );

  await link.drone.send(new Attitude(1000, 0.1, -0.05, 1.57, 0, 0, 0));

  await delay(50);
  subscription.cancel();

  console.log(`Received ${{attitudeSamples.length}} ATTITUDE samples via listenMessage`);
  if (attitudeSamples.length > 0) {{
    const sample = attitudeSamples[0];
    console.log(`  roll=${{sample.roll}} pitch=${{sample.pitch}} yaw=${{sample.yaw}}`);
  }}

  await closeVirtualLink({{ bus: link.bus, gcs: link.gcs, drone: link.drone }});
}}

main();
"#
    )
}
