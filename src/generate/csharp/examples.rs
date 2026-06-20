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
    (
        "protocols_common.cs",
        include_str!("../../../templates/csharp/examples/protocols_common.cs"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::examples::{
        ALL_EXAMPLE_SUFFIXES, LOW_LEVEL_EXAMPLE_SUFFIXES, PROTOCOL_EXAMPLE_SUFFIXES,
    };

    #[test]
    fn example_suffixes_match_shared_constants() {
        let low_level: Vec<_> = LOW_LEVEL_EXAMPLES.iter().map(|(s, _)| *s).collect();
        let protocol: Vec<_> = PROTOCOL_EXAMPLES.iter().map(|(s, _)| *s).collect();
        let all: Vec<_> = LOW_LEVEL_EXAMPLES
            .iter()
            .chain(PROTOCOL_EXAMPLES.iter())
            .map(|(s, _)| *s)
            .collect();

        assert_eq!(low_level, LOW_LEVEL_EXAMPLE_SUFFIXES);
        assert_eq!(protocol, PROTOCOL_EXAMPLE_SUFFIXES);
        assert_eq!(all, ALL_EXAMPLE_SUFFIXES);
    }
}

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
                LOW_LEVEL_EXAMPLES
                    .iter()
                    .chain(PROTOCOL_EXAMPLES.iter())
                    .flat_map(move |(suffix, render)| {
                        let stem = stem.clone();
                        let suffix = *suffix;
                        [
                            ExampleFile {
                                relative_path: PathBuf::from(format!("{stem}_{suffix}.cs")),
                                content: render(&stem),
                            },
                            ExampleFile {
                                relative_path: PathBuf::from(format!("{stem}_{suffix}.csproj")),
                                content: super::runtime::render_example_csproj(&stem, suffix),
                            },
                        ]
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
        paramValue: param.Value,
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
            $"{{Common.ParamIdToString(parsedValue.ParamId)}}={{parsedValue.paramValue}}");
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
    paramValue: 1,
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

fn render_protocol_mission_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Mission protocol example for the `{dialect_stem}` dialect.
/// </summary>
/// <remarks>Uses <see cref="VirtualMavlinkBus"/> via <see cref="ProtocolsCommon.CreateVirtualLink"/>.</remarks>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var missionServer = new MissionServer(link.Drone);
await using var commandServer = new CommandServer(link.Drone);
var missionProtocol = new MissionProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var plan = new[]
{{
    MissionItems.Waypoint(
        0,
        47.397742,
        8.545594,
        50,
        ProtocolsCommon.DroneSystemId,
        ProtocolsCommon.DroneComponentId),
    MissionItems.Waypoint(
        1,
        47.398000,
        8.546000,
        50,
        ProtocolsCommon.DroneSystemId,
        ProtocolsCommon.DroneComponentId),
}};

var uploadResult = await missionProtocol.UploadAsync(
    plan,
    onProgress: (sent, total, item) =>
        Console.WriteLine($"Upload progress {{sent}}/{{total}} seq={{item.Seq}} cmd={{item.Command}}"));
Console.WriteLine($"Mission upload result: {{uploadResult}}");
Console.WriteLine($"Vehicle stored {{missionServer.Items.Count}} items");

var downloaded = await missionProtocol.DownloadAsync(
    onProgress: (received, total, item) =>
        Console.WriteLine($"Download progress {{received}}/{{total}} seq={{item.Seq}}"));
Console.WriteLine($"Downloaded {{downloaded.Count}} mission items");

var commandProtocol = new CommandProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);
var setCurrent = await missionProtocol.SetCurrentWithCommandAsync(
    0,
    command: commandProtocol);
Console.WriteLine($"Set current seq={{setCurrent.Sequence}} ack={{setCurrent.CommandAck?.Result}}");

var clearResult = await missionProtocol.ClearAsync();
Console.WriteLine($"Mission clear result: {{clearResult}}");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
"#
    )
}

fn render_protocol_parameters_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Parameter protocol example for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var parameterServer = new ParameterServer(
    link.Drone,
    new Dictionary<string, ParamStoreEntry>
    {{
        ["SYSID_THISMAV"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
        ["SYSID_MYGCS"] = new(255, MavParamType.MAV_PARAM_TYPE_INT32),
        ["COMPASS_ENABLE"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
    }});

var parameterProtocol = new ParameterProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var allParams = await parameterProtocol.FetchAllAsync(
    onProgress: (entry, received, expected) =>
        Console.WriteLine($"  [{{received}}/{{expected}}] {{entry.Id}}={{entry.Value}}"));
Console.WriteLine(
    $"Fetched {{allParams.Count}} parameters (cache size={{parameterProtocol.Cache.Count}})");

var single = await parameterProtocol.ReadByNameAsync("SYSID_THISMAV");
Console.WriteLine($"Read SYSID_THISMAV={{single.Value}}");

var updated = await parameterProtocol.WriteByNameAsync("COMPASS_ENABLE", 0);
Console.WriteLine($"Wrote COMPASS_ENABLE={{updated.Value}} ({{updated.Type}})");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
"#
    )
}

fn render_protocol_command_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Command protocol example for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var commandServer = new CommandServer(
    link.Drone,
    onCommandLong: command =>
    {{
        Console.WriteLine(
            $"Vehicle received COMMAND_LONG: {{command.Command}} " +
            $"p1={{command.Param1}} p2={{command.Param2}}");
        return Task.FromResult(MavResult.MAV_RESULT_ACCEPTED);
    }});

var commandProtocol = new CommandProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var intervalAck = await commandProtocol.SetMessageIntervalAsync(Attitude.MsgId, 100000);
Console.WriteLine($"SET_MESSAGE_INTERVAL ack: {{intervalAck.Result}}");

var requestAck = await commandProtocol.RequestMessageAsync(Attitude.MsgId);
Console.WriteLine($"REQUEST_MESSAGE ack: {{requestAck.Result}}");

var armAck = await commandProtocol.ArmAsync();
Console.WriteLine($"ARM ack: {{armAck.Result}}");

var disarmAck = await commandProtocol.DisarmAsync();
Console.WriteLine($"DISARM ack: {{disarmAck.Result}}");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
"#
    )
}

fn render_protocol_heartbeat_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Heartbeat protocol example for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

var gcsPublisher = new HeartbeatPublisher(
    link.Gcs,
    HeartbeatTemplates.Gcs(dialect.Version),
    TimeSpan.FromMilliseconds(500));

var dronePublisher = new HeartbeatPublisher(
    link.Drone,
    HeartbeatTemplates.Autopilot(dialect.Version),
    TimeSpan.FromMilliseconds(500));

var gcsMonitor = new HeartbeatMonitor(link.Gcs, TimeSpan.FromSeconds(2));

gcsMonitor.Start();
gcsPublisher.Start();
dronePublisher.Start();

var vehicle = await gcsMonitor.WaitForVehicleAsync(
    excludeSystemIds: new HashSet<byte> {{ ProtocolsCommon.GcsSystemId }},
    timeout: TimeSpan.FromSeconds(5));
Console.WriteLine($"Vehicle discovered: {{vehicle}}");
Console.WriteLine($"Drone online: {{gcsMonitor.IsOnline(vehicle)}}");
var state = gcsMonitor.StateFor(vehicle);
if (state is not null)
{{
    Console.WriteLine(
        $"Drone heartbeat: type={{state.Heartbeat.Type}} status={{state.Heartbeat.SystemStatus}}");
}}

dronePublisher.Stop();
await Task.Delay(TimeSpan.FromMilliseconds(2500));
Console.WriteLine($"Drone online after stop: {{gcsMonitor.IsOnline(vehicle)}}");

await gcsMonitor.StopAsync();
gcsPublisher.Stop();

await ProtocolsCommon.CloseVirtualLinkAsync(link);
"#
    )
}

fn render_protocol_vehicle_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// MavlinkGcs / MavlinkVehicleClient facade example for `{dialect_stem}`.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var bus = new VirtualMavlinkBus();
var gcsLink = bus.CreateEndpoint();
var droneLink = bus.CreateEndpoint();

await using var gcs = MavlinkGcs.Connect(dialect, gcsLink);

var droneSession = new MavlinkSession(
    dialect,
    droneLink,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var dronePublisher = new HeartbeatPublisher(
    droneSession,
    HeartbeatTemplates.Autopilot(dialect.Version),
    TimeSpan.FromMilliseconds(500));

await using var parameterServer = new ParameterServer(
    droneSession,
    new Dictionary<string, ParamStoreEntry>
    {{
        ["SYSID_THISMAV"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
    }});

await using var commandServer = new CommandServer(droneSession);

gcs.Start();
dronePublisher.Start();

var client = await gcs.WaitForVehicleAsync(
    excludeSystemIds: new HashSet<byte> {{ ProtocolsCommon.GcsSystemId }});
Console.WriteLine($"Connected to vehicle {{client.Vehicle}}");

var parameters = await client.Parameters.FetchAllAsync();
Console.WriteLine($"Vehicle has {{parameters.Count}} parameters");

var ack = await client.Command.RequestMessageAsync(Heartbeat.MsgId);
Console.WriteLine($"REQUEST_MESSAGE ack: {{ack.Result}}");

dronePublisher.Stop();
await droneSession.CloseAsync();
await gcs.CloseAsync();
await bus.CloseAllAsync();
"#
    )
}

fn render_protocol_subscribe_example(dialect_stem: &str) -> String {
    let dialect_class = dialect_class_name(dialect_stem);

    format!(
        r#"/// <summary>
/// Typed message subscription example for the `{dialect_stem}` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new {dialect_class}();
var link = ProtocolsCommon.CreateVirtualLink(dialect);
var vehicle = new MavlinkNode(ProtocolsCommon.DroneSystemId, ProtocolsCommon.DroneComponentId);

var attitudeSamples = new List<Attitude>();
using var subscription = link.Gcs.ListenMessage<Attitude>(
    (message, frame) => attitudeSamples.Add(message),
    fromSystemId: vehicle.SystemId);

await link.Drone.SendAsync(
    new Attitude(
        TimeBootMs: 1000,
        Roll: 0.1f,
        Pitch: -0.05f,
        Yaw: 1.57f,
        Rollspeed: 0,
        Pitchspeed: 0,
        Yawspeed: 0));

await Task.Delay(TimeSpan.FromMilliseconds(50));
subscription.Cancel();

Console.WriteLine($"Received {{attitudeSamples.Count}} ATTITUDE samples via ListenMessage");
if (attitudeSamples.Count > 0)
{{
    var sample = attitudeSamples[0];
    Console.WriteLine($"  roll={{sample.Roll}} pitch={{sample.Pitch}} yaw={{sample.Yaw}}");
}}

await ProtocolsCommon.CloseVirtualLinkAsync(link);
"#
    )
}
