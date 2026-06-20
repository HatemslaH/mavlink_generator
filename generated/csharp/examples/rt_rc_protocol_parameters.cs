/// <summary>
/// Parameter protocol example for the `rt_rc` dialect.
/// </summary>
using Mavlink;
using Mavlink.Dialects;

var dialect = new MavlinkDialectRt_rc();
var link = ProtocolsCommon.CreateVirtualLink(dialect);

await using var parameterServer = new ParameterServer(
    link.Drone,
    new Dictionary<string, ParamStoreEntry>
    {
        ["SYSID_THISMAV"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
        ["SYSID_MYGCS"] = new(255, MavParamType.MAV_PARAM_TYPE_INT32),
        ["COMPASS_ENABLE"] = new(1, MavParamType.MAV_PARAM_TYPE_INT32),
    });

var parameterProtocol = new ParameterProtocol(
    link.Gcs,
    ProtocolsCommon.DroneSystemId,
    ProtocolsCommon.DroneComponentId);

var allParams = await parameterProtocol.FetchAllAsync(
    onProgress: (entry, received, expected) =>
        Console.WriteLine($"  [{received}/{expected}] {entry.Id}={entry.Value}"));
Console.WriteLine(
    $"Fetched {allParams.Count} parameters (cache size={parameterProtocol.Cache.Count})");

var single = await parameterProtocol.ReadByNameAsync("SYSID_THISMAV");
Console.WriteLine($"Read SYSID_THISMAV={single.Value}");

var updated = await parameterProtocol.WriteByNameAsync("COMPASS_ENABLE", 0);
Console.WriteLine($"Wrote COMPASS_ENABLE={updated.Value} ({updated.Type})");

await ProtocolsCommon.CloseVirtualLinkAsync(link);
