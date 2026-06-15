use std::path::PathBuf;

use crate::generate::examples::{ExampleFile, LanguageExampleGenerator};
use crate::xml::capitalize;

pub struct CSharpExampleGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/csharp/examples/README.md"),
    ),
    (
        "common.cs",
        include_str!("../../../templates/csharp/examples/common.cs"),
    ),
];

const GENERATED_EXAMPLES: &[(&str, fn(&str) -> String)] = &[
    ("heartbeat", render_heartbeat_example),
    ("mission_upload", render_mission_upload_example),
    ("request_telemetry", render_request_telemetry_example),
    ("request_parameters", render_request_parameters_example),
];

const EXAMPLE_SUFFIXES: &[&str] = &[
    "heartbeat",
    "mission_upload",
    "request_telemetry",
    "request_parameters",
];

impl LanguageExampleGenerator for CSharpExampleGenerator {
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
                let examples = GENERATED_EXAMPLES.iter().map({
                    let stem = stem.clone();
                    move |(suffix, render)| ExampleFile {
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.cs")),
                        content: render(&stem),
                    }
                });
                let projects = EXAMPLE_SUFFIXES.iter().map({
                    let stem = stem.clone();
                    move |suffix| ExampleFile {
                        relative_path: PathBuf::from(format!("{stem}_{suffix}.csproj")),
                        content: super::runtime::render_example_csproj(&stem, suffix),
                    }
                });
                examples.chain(projects)
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
        r#"/// <summary>
/// Example for the `{dialect_stem}` dialect: serialize a Heartbeat frame and parse it back.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();

var heartbeat = new Heartbeat(
    CustomMode: 0,
    Type: MavType.MAV_TYPE_QUADROTOR,
    Autopilot: MavAutopilot.MAV_AUTOPILOT_PX4,
    BaseMode: 0,
    SystemStatus: MavState.MAV_STATE_ACTIVE,
    MavlinkVersion: dialect.Version
);

var frame = Common.FrameFromGcs(heartbeat);
var wire = frame.Serialize();
Common.LogFrame("GCS ->", frame);
Console.WriteLine($"Serialized HEARTBEAT ({{wire.Length}} bytes)");

var parsed = Common.RoundTripMessage(dialect, heartbeat);
if (parsed is Heartbeat parsedHeartbeat)
{{
    Console.WriteLine(
        $"Parsed HEARTBEAT type={{parsedHeartbeat.Type}} status={{parsedHeartbeat.SystemStatus}}");
}}
"#
    )
}

fn render_mission_upload_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Virtual mission upload for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var missionType = MavMissionType.MAV_MISSION_TYPE_MISSION;

var missionItems = new[]
{{
    new MissionItem(
        Param1: 0,
        Param2: 2,
        Param3: 0,
        Param4: 0,
        X: 47.397742f,
        Y: 8.545594f,
        Z: 50,
        Seq: 0,
        Command: MavCmd.MAV_CMD_NAV_WAYPOINT,
        TargetSystem: Common.DroneSystemId,
        TargetComponent: Common.DroneComponentId,
        Frame: MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
        Current: 0,
        Autocontinue: 1,
        MissionType: missionType),
    new MissionItem(
        Param1: 0,
        Param2: 2,
        Param3: 0,
        Param4: 0,
        X: 47.398000f,
        Y: 8.546000f,
        Z: 50,
        Seq: 1,
        Command: MavCmd.MAV_CMD_NAV_WAYPOINT,
        TargetSystem: Common.DroneSystemId,
        TargetComponent: Common.DroneComponentId,
        Frame: MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT,
        Current: 0,
        Autocontinue: 1,
        MissionType: missionType),
}};

var seq = 0;

var count = new MissionCount(
    Count: missionItems.Length,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    MissionType: missionType);
var countFrame = Common.FrameFromGcs(count, 1);
Common.LogFrame("GCS ->", countFrame);
Common.RoundTripMessage(dialect, count);

while (seq < missionItems.Length)
{{
    var request = new MissionRequest(
        Seq: (ushort)seq,
        TargetSystem: Common.GcsSystemId,
        TargetComponent: Common.GcsComponentId,
        MissionType: missionType);
    var requestFrame = Common.FrameFromDrone(request, (byte)(seq + 10));
    Common.LogFrame("Drone ->", requestFrame);
    Common.RoundTripMessage(dialect, request);

    var item = missionItems[seq];
    var itemFrame = Common.FrameFromGcs(item, (byte)(seq + 20));
    Common.LogFrame("GCS ->", itemFrame);
    var parsedItem = Common.RoundTripMessage(dialect, item);
    if (parsedItem is MissionItem uploaded)
    {{
        Console.WriteLine($"  uploaded seq={{uploaded.Seq}} cmd={{uploaded.Command}}");
    }}

    seq++;
}}

var ack = new MissionAck(
    TargetSystem: Common.GcsSystemId,
    TargetComponent: Common.GcsComponentId,
    Type: MavMissionResult.MAV_MISSION_ACCEPTED,
    MissionType: missionType);
var ackFrame = Common.FrameFromDrone(ack, 99);
Common.LogFrame("Drone ->", ackFrame);
var parsedAck = Common.RoundTripMessage(dialect, ack);
if (parsedAck is MissionAck missionAck)
{{
    Console.WriteLine($"Mission upload complete: {{missionAck.Type}}");
}}
"#
    )
}

fn render_request_telemetry_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Virtual telemetry request for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();

var setInterval = new CommandLong(
    Param1: Attitude.MsgId,
    Param2: 100000,
    Param3: 0,
    Param4: 0,
    Param5: 0,
    Param6: 0,
    Param7: 0,
    Command: MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    Confirmation: 0);
var intervalFrame = Common.FrameFromGcs(setInterval, 1);
Common.LogFrame("GCS ->", intervalFrame);
var parsedInterval = Common.RoundTripMessage(dialect, setInterval);
if (parsedInterval is CommandLong interval)
{{
    Console.WriteLine(
        $"  SET_MESSAGE_INTERVAL msgId={{(int)interval.Param1}} interval_us={{(int)interval.Param2}}");
}}

var requestOnce = new CommandLong(
    Param1: Attitude.MsgId,
    Param2: 0,
    Param3: 0,
    Param4: 0,
    Param5: 0,
    Param6: 0,
    Param7: 0,
    Command: MavCmd.MAV_CMD_REQUEST_MESSAGE,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    Confirmation: 0);
var onceFrame = Common.FrameFromGcs(requestOnce, 2);
Common.LogFrame("GCS ->", onceFrame);
Common.RoundTripMessage(dialect, requestOnce);

var attitude = new Attitude(
    TimeBootMs: 12345,
    Roll: 0.01f,
    Pitch: -0.02f,
    Yaw: 1.57f,
    Rollspeed: 0,
    Pitchspeed: 0,
    Yawspeed: 0);
var telemetryFrame = Common.FrameFromDrone(attitude, 3);
Common.LogFrame("Drone ->", telemetryFrame);
var parsedAttitude = Common.RoundTripMessage(dialect, attitude);
if (parsedAttitude is Attitude parsed)
{{
    Console.WriteLine(
        $"  ATTITUDE roll={{parsed.Roll}} pitch={{parsed.Pitch}} yaw={{parsed.Yaw}}");
}}
"#
    )
}

fn render_request_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Virtual parameter service for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();

var listRequest = new ParamRequestList(
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId);
var listFrame = Common.FrameFromGcs(listRequest, 1);
Common.LogFrame("GCS ->", listFrame);
Common.RoundTripMessage(dialect, listRequest);

var simulatedParams = new (string Id, float Value, ushort Index)[]
{{
    ("SYSID_THISMAV", 1, 0),
    ("SYSID_MYGCS", 255, 1),
    ("COMPASS_ENABLE", 1, 2),
}};

foreach (var param in simulatedParams)
{{
    var value = new ParamValue(
        ParamValue: param.Value,
        ParamCount: (ushort)simulatedParams.Length,
        ParamIndex: param.Index,
        ParamId: Common.ParamIdFromString(param.Id),
        ParamType: MavParamType.MAV_PARAM_TYPE_REAL32);
    var valueFrame = Common.FrameFromDrone(value, (byte)(param.Index + 10));
    Common.LogFrame("Drone ->", valueFrame);
    var parsed = Common.RoundTripMessage(dialect, value);
    if (parsed is ParamValue parsedValue)
    {{
        Console.WriteLine(
            $"  PARAM_VALUE [{{param.Index + 1}}/{{simulatedParams.Length}}] " +
            $"{{Common.ParamIdToString(parsedValue.ParamId)}}={{parsedValue.ParamValue}}");
    }}
}}

var paramName = "SYSID_THISMAV";
var readRequest = new ParamRequestRead(
    ParamIndex: ushort.MaxValue,
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId,
    ParamId: Common.ParamIdFromString(paramName));
var readFrame = Common.FrameFromGcs(readRequest, 50);
Common.LogFrame("GCS ->", readFrame);
var parsedRead = Common.RoundTripMessage(dialect, readRequest);
if (parsedRead is ParamRequestRead requestRead)
{{
    Console.WriteLine($"  PARAM_REQUEST_READ id={{Common.ParamIdToString(requestRead.ParamId)}}");
}}

var singleValue = new ParamValue(
    ParamValue: 1,
    ParamCount: (ushort)simulatedParams.Length,
    ParamIndex: 0,
    ParamId: Common.ParamIdFromString(paramName),
    ParamType: MavParamType.MAV_PARAM_TYPE_REAL32);
var singleFrame = Common.FrameFromDrone(singleValue, 51);
Common.LogFrame("Drone ->", singleFrame);
Common.RoundTripMessage(dialect, singleValue);
"#
    )
}
