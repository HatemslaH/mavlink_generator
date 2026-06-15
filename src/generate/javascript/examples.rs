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
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
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
                GENERATED_EXAMPLES
                    .iter()
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
