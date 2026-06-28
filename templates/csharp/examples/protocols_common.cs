using Mavlink;
using Mavlink.Dialects;

/// <summary>Shared helpers for MAVLink protocol-layer examples.</summary>
public static class ProtocolsCommon
{
    public const byte GcsSystemId = 255;
    public const byte GcsComponentId = 190;

    public const byte DroneSystemId = 1;
    public const byte DroneComponentId = 1;

    public sealed record VirtualLink(
        VirtualMavlinkBus Bus,
        MavlinkSession Gcs,
        MavlinkSession Drone,
        MavlinkDialect Dialect);

    public static VirtualLink CreateVirtualLink(MavlinkDialect dialect)
    {
        var bus = new VirtualMavlinkBus();
        var gcsLink = bus.CreateEndpoint();
        var droneLink = bus.CreateEndpoint();

        var gcs = new MavlinkSession(dialect, gcsLink, GcsSystemId, GcsComponentId);
        var drone = new MavlinkSession(dialect, droneLink, DroneSystemId, DroneComponentId);
        return new VirtualLink(bus, gcs, drone, dialect);
    }

    public static async Task CloseVirtualLinkAsync(VirtualLink link)
    {
        await link.Gcs.CloseAsync().ConfigureAwait(false);
        await link.Drone.CloseAsync().ConfigureAwait(false);
        await link.Bus.CloseAllAsync().ConfigureAwait(false);
    }
}
