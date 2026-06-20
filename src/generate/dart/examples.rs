use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct DartExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/dart/examples/README.md"),
    ),
    (
        "common.dart",
        include_str!("../../../templates/dart/examples/common.dart"),
    ),
    (
        "protocols_common.dart",
        include_str!("../../../templates/dart/examples/protocols_common.dart"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
    ("protocol_mission", render_protocol_mission_example),
    ("protocol_parameters", render_protocol_parameters_example),
    ("protocol_command", render_protocol_command_example),
    ("protocol_heartbeat", render_protocol_heartbeat_example),
];

impl LanguageExampleGenerator for DartExampleGenerator {
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
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.dart")),
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
        r#"// ignore_for_file: avoid_print

import 'common.dart';

/// Example for the `{dialect_stem}` dialect: serialize a [Heartbeat] frame and
/// parse it back with [{dialect_class}].
void main() {{
  final dialect = {dialect_class}();

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
  print('Serialized HEARTBEAT (${{bytes.length}} bytes)');

  final parsed = roundTripMessage(dialect, heartbeat);
  if (parsed is Heartbeat) {{
    print('Parsed HEARTBEAT type=${{parsed.type}} status=${{parsed.systemStatus}}');
  }}
}}
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual mission upload for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/mission.html upload sequence:
/// GCS -> MISSION_COUNT -> Drone -> MISSION_REQUEST* -> GCS -> MISSION_ITEM* -> Drone -> MISSION_ACK
void main() {{
  final dialect = {dialect_class}();
  const missionType = MavMissionType.mavMissionTypeMission;

  final missionItems = <MissionItem>[
    MissionItem(
      param1: 0,
      param2: 2,
      param3: 0,
      param4: 0,
      x: 47.397742,
      y: 8.545594,
      z: 50,
      seq: 0,
      command: MavCmd.mavCmdNavWaypoint,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
      frame: MavFrame.mavFrameGlobalRelativeAlt,
      current: 0,
      autocontinue: 1,
      missionType: missionType,
    ),
    MissionItem(
      param1: 0,
      param2: 2,
      param3: 0,
      param4: 0,
      x: 47.398000,
      y: 8.546000,
      z: 50,
      seq: 1,
      command: MavCmd.mavCmdNavWaypoint,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
      frame: MavFrame.mavFrameGlobalRelativeAlt,
      current: 0,
      autocontinue: 1,
      missionType: missionType,
    ),
  ];

  var seq = 0;

  // 1. GCS announces mission size.
  final count = MissionCount(
    count: missionItems.length,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    missionType: missionType,
  );
  final countFrame = frameFromGcs(count, sequence: 1);
  logFrame('GCS ->', countFrame);
  roundTripMessage(dialect, count);

  // 2. Drone requests each mission item, GCS responds.
  while (seq < missionItems.length) {{
    final request = MissionRequest(
      seq: seq,
      targetSystem: gcsSystemId,
      targetComponent: gcsComponentId,
      missionType: missionType,
    );
    final requestFrame = frameFromDrone(request, sequence: seq + 10);
    logFrame('Drone ->', requestFrame);
    roundTripMessage(dialect, request);

    final item = missionItems[seq];
    final itemFrame = frameFromGcs(item, sequence: seq + 20);
    logFrame('GCS ->', itemFrame);
    final parsedItem = roundTripMessage(dialect, item);
    if (parsedItem is MissionItem) {{
      print('  uploaded seq=${{parsedItem.seq}} cmd=${{parsedItem.command}}');
    }}

    seq++;
  }}

  // 3. Drone accepts the mission.
  final ack = MissionAck(
    targetSystem: gcsSystemId,
    targetComponent: gcsComponentId,
    type: MavMissionResult.mavMissionAccepted,
    missionType: missionType,
  );
  final ackFrame = frameFromDrone(ack, sequence: 99);
  logFrame('Drone ->', ackFrame);
  final parsedAck = roundTripMessage(dialect, ack);
  if (parsedAck is MissionAck) {{
    print('Mission upload complete: ${{parsedAck.type}}');
  }}
}}
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual telemetry request for the `{dialect_stem}` dialect.
///
/// Uses COMMAND_LONG with MAV_CMD_SET_MESSAGE_INTERVAL (preferred) and
/// MAV_CMD_REQUEST_MESSAGE (one-shot), per MAVLink command protocol.
void main() {{
  final dialect = {dialect_class}();

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
  if (parsedInterval is CommandLong) {{
    print(
      '  SET_MESSAGE_INTERVAL msgId=${{parsedInterval.param1.toInt()}} '
      'interval_us=${{parsedInterval.param2.toInt()}}',
    );
  }}

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
  if (parsedAttitude is Attitude) {{
    print(
      '  ATTITUDE roll=${{parsedAttitude.roll}} '
      'pitch=${{parsedAttitude.pitch}} yaw=${{parsedAttitude.yaw}}',
    );
  }}
}}
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual parameter service for the `{dialect_stem}` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
void main() {{
  final dialect = {dialect_class}();

  // 1. GCS requests the full onboard parameter set.
  final listRequest = ParamRequestList(
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );
  final listFrame = frameFromGcs(listRequest, sequence: 1);
  logFrame('GCS ->', listFrame);
  roundTripMessage(dialect, listRequest);

  // 2. Drone responds with PARAM_VALUE messages (simulated subset).
  final simulatedParams = <({{String id, double value, int index}})>[
    (id: 'SYSID_THISMAV', value: 1, index: 0),
    (id: 'SYSID_MYGCS', value: 255, index: 1),
    (id: 'COMPASS_ENABLE', value: 1, index: 2),
  ];

  for (final param in simulatedParams) {{
    final value = ParamValue(
      paramValue: param.value,
      paramCount: simulatedParams.length,
      paramIndex: param.index,
      paramId: paramIdFromString(param.id),
      paramType: MavParamType.mavParamTypeReal32,
    );
    final valueFrame = frameFromDrone(value, sequence: param.index + 10);
    logFrame('Drone ->', valueFrame);
    final parsed = roundTripMessage(dialect, value);
    if (parsed is ParamValue) {{
      print(
        '  PARAM_VALUE [${{param.index + 1}}/${{simulatedParams.length}}] '
        '${{paramIdToString(parsed.paramId)}}=${{parsed.paramValue}}',
      );
    }}
  }}

  // 3. GCS requests one parameter by name (param_index = -1).
  const paramName = 'SYSID_THISMAV';
  final readRequest = ParamRequestRead(
    paramIndex: -1,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    paramId: paramIdFromString(paramName),
  );
  final readFrame = frameFromGcs(readRequest, sequence: 50);
  logFrame('GCS ->', readFrame);
  final parsedRead = roundTripMessage(dialect, readRequest);
  if (parsedRead is ParamRequestRead) {{
    print('  PARAM_REQUEST_READ id=${{paramIdToString(parsedRead.paramId)}}');
  }}

  // 4. Drone answers with the matching PARAM_VALUE.
  final singleValue = ParamValue(
    paramValue: 1,
    paramCount: simulatedParams.length,
    paramIndex: 0,
    paramId: paramIdFromString(paramName),
    paramType: MavParamType.mavParamTypeReal32,
  );
  final singleFrame = frameFromDrone(singleValue, sequence: 51);
  logFrame('Drone ->', singleFrame);
  roundTripMessage(dialect, singleValue);
}}
"#
    )
}

fn render_protocol_mission_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Mission protocol example for the `{dialect_stem}` dialect.
///
/// Uses [MissionProtocol] on the GCS side and [MissionServer] on the vehicle
/// side over a transport-agnostic in-memory [VirtualMavlinkBus].
Future<void> main() async {{
  final dialect = {dialect_class}();
  final link = createVirtualLink(dialect);

  final missionServer = MissionServer(session: link.drone);
  final missionProtocol = MissionProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final plan = <MissionItemInt>[
    MissionItems.waypoint(
      seq: 0,
      latitude: 47.397742,
      longitude: 8.545594,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    ),
    MissionItems.waypoint(
      seq: 1,
      latitude: 47.398000,
      longitude: 8.546000,
      altitude: 50,
      targetSystem: droneSystemId,
      targetComponent: droneComponentId,
    ),
  ];

  final uploadResult = await missionProtocol.upload(plan);
  print('Mission upload result: $uploadResult');
  print('Vehicle stored ${{missionServer.items.length}} items');

  final downloaded = await missionProtocol.download();
  print('Downloaded ${{downloaded.length}} mission items');

  final clearResult = await missionProtocol.clear();
  print('Mission clear result: $clearResult');

  await missionServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}}
"#
    )
}

fn render_protocol_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Parameter protocol example for the `{dialect_stem}` dialect.
///
/// Uses [ParameterProtocol] on the GCS side and [ParameterServer] on the
/// vehicle side. The link is transport-agnostic and can be swapped for USB,
/// UDP, TCP, or any custom [MavlinkLink] implementation.
Future<void> main() async {{
  final dialect = {dialect_class}();
  final link = createVirtualLink(dialect);

  final parameterServer = ParameterServer(
    session: link.drone,
    initialValues: {{
      'SYSID_THISMAV': (value: 1, type: MavParamType.mavParamTypeInt32),
      'SYSID_MYGCS': (value: 255, type: MavParamType.mavParamTypeInt32),
      'COMPASS_ENABLE': (value: 1, type: MavParamType.mavParamTypeInt32),
    }},
  );

  final parameterProtocol = ParameterProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final allParams = await parameterProtocol.fetchAll();
  print('Fetched ${{allParams.length}} parameters:');
  for (final param in allParams) {{
    print('  ${{param.id}}=${{param.value}} (${{param.type}})');
  }}

  final single = await parameterProtocol.readByName('SYSID_THISMAV');
  print('Read SYSID_THISMAV=${{single.value}}');

  final updated = await parameterProtocol.write(
    name: 'COMPASS_ENABLE',
    value: 0,
    type: MavParamType.mavParamTypeInt32,
  );
  print('Wrote COMPASS_ENABLE=${{updated.value}}');

  await parameterServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}}
"#
    )
}

fn render_protocol_command_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Command protocol example for the `{dialect_stem}` dialect.
///
/// Uses [CommandProtocol] on the GCS side and [CommandServer] on the vehicle
/// side. Demonstrates message interval setup and one-shot telemetry requests.
Future<void> main() async {{
  final dialect = {dialect_class}();
  final link = createVirtualLink(dialect);

  final commandServer = CommandServer(
    session: link.drone,
    onCommandLong: (command) async {{
      print(
        'Vehicle received COMMAND_LONG: ${{command.command}} '
        'p1=${{command.param1}} p2=${{command.param2}}',
      );
      return MavResult.mavResultAccepted;
    }},
  );

  final commandProtocol = CommandProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final intervalAck = await commandProtocol.setMessageInterval(
    Attitude.msgId,
    100000,
  );
  print('SET_MESSAGE_INTERVAL ack: ${{intervalAck.result}}');

  final requestAck = await commandProtocol.requestMessage(Attitude.msgId);
  print('REQUEST_MESSAGE ack: ${{requestAck.result}}');

  await commandServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}}
"#
    )
}

fn render_protocol_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"// ignore_for_file: avoid_print

import 'dart:async';

import 'protocols_common.dart';

/// Heartbeat protocol example for the `{dialect_stem}` dialect.
///
/// Uses [HeartbeatPublisher] to send heartbeats and [HeartbeatMonitor] to track
/// remote node connectivity over a transport-agnostic link.
Future<void> main() async {{
  final dialect = {dialect_class}();
  final link = createVirtualLink(dialect);
  final droneNode = MavlinkNode(droneSystemId, droneComponentId);

  final gcsPublisher = HeartbeatPublisher(
    session: link.gcs,
    heartbeat: HeartbeatTemplates.gcs(mavlinkVersion: dialect.version),
    interval: const Duration(milliseconds: 500),
  );

  final dronePublisher = HeartbeatPublisher(
    session: link.drone,
    heartbeat: HeartbeatTemplates.autopilot(mavlinkVersion: dialect.version),
    interval: const Duration(milliseconds: 500),
  );

  final gcsMonitor = HeartbeatMonitor(
    session: link.gcs,
    timeout: const Duration(seconds: 2),
    watch: {{droneNode}},
  );

  final connectionEvents = <String>[];
  final monitorSub = gcsMonitor.onConnected.listen(
    (node) => connectionEvents.add('connected $node'),
  );
  final disconnectSub = gcsMonitor.onDisconnected.listen(
    (node) => connectionEvents.add('disconnected $node'),
  );

  gcsMonitor.start();
  gcsPublisher.start();
  dronePublisher.start();

  await Future<void>.delayed(const Duration(milliseconds: 1200));
  print('Drone online: ${{gcsMonitor.isOnline(droneNode)}}');
  final state = gcsMonitor.stateFor(droneNode);
  if (state != null) {{
    print(
      'Drone heartbeat: type=${{state.heartbeat.type}} '
      'status=${{state.heartbeat.systemStatus}}',
    );
  }}

  dronePublisher.stop();
  await Future<void>.delayed(const Duration(milliseconds: 2500));
  print('Drone online after stop: ${{gcsMonitor.isOnline(droneNode)}}');
  print('Events: ${{connectionEvents.join(', ')}}');

  await monitorSub.cancel();
  await disconnectSub.cancel();
  await gcsMonitor.stop();
  gcsPublisher.stop();

  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}}
"#
    )
}
