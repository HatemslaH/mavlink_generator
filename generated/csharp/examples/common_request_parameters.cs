/// <summary>
/// Virtual parameter service for the `common` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectCommon();

var listRequest = new ParamRequestList(
    TargetSystem: Common.DroneSystemId,
    TargetComponent: Common.DroneComponentId);
var listFrame = Common.FrameFromGcs(listRequest, 1);
Common.LogFrame("GCS ->", listFrame);
Common.RoundTripMessage(dialect, listRequest);

var simulatedParams = new (string Id, float Value, ushort Index)[]
{
    ("SYSID_THISMAV", 1, 0),
    ("SYSID_MYGCS", 255, 1),
    ("COMPASS_ENABLE", 1, 2),
};

foreach (var param in simulatedParams)
{
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
    {
        Console.WriteLine(
            $"  PARAM_VALUE [{param.Index + 1}/{simulatedParams.Length}] " +
            $"{Common.ParamIdToString(parsedValue.ParamId)}={parsedValue.paramValue}");
    }
}

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
{
    Console.WriteLine($"  PARAM_REQUEST_READ id={Common.ParamIdToString(requestRead.ParamId)}");
}

var singleValue = new ParamValue(
    paramValue: 1,
    ParamCount: (ushort)simulatedParams.Length,
    ParamIndex: 0,
    ParamId: Common.ParamIdFromString(paramName),
    ParamType: MavParamType.MAV_PARAM_TYPE_REAL32);
var singleFrame = Common.FrameFromDrone(singleValue, 51);
Common.LogFrame("Drone ->", singleFrame);
Common.RoundTripMessage(dialect, singleValue);
