using Mavlink.Dialects;

namespace Mavlink;

/// <summary>GCS-side MAVLink command protocol client.</summary>
public sealed class CommandProtocol
{
    private readonly MavlinkSession _session;

    public CommandProtocol(
        MavlinkSession session,
        byte targetSystem,
        byte targetComponent,
        TimeSpan? defaultTimeout = null)
    {
        _session = session;
        TargetSystem = targetSystem;
        TargetComponent = targetComponent;
        DefaultTimeout = defaultTimeout ?? TimeSpan.FromSeconds(5);
    }

    public byte TargetSystem { get; }

    public byte TargetComponent { get; }

    public TimeSpan DefaultTimeout { get; }

    public async Task<CommandAck> SendLongAsync(
        CommandLong command,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        await _session.SendAsync(command, CancellationToken.None).ConfigureAwait(false);
        return await WaitForAckAsync(command.Command, timeout, cancel).ConfigureAwait(false);
    }

    public async Task<CommandAck> SendIntAsync(
        CommandInt command,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        await _session.SendAsync(command, CancellationToken.None).ConfigureAwait(false);
        return await WaitForAckAsync(command.Command, timeout, cancel).ConfigureAwait(false);
    }

    public Task<CommandAck> CommandLongAsync(
        MavCmd command,
        float param1 = 0,
        float param2 = 0,
        float param3 = 0,
        float param4 = 0,
        float param5 = 0,
        float param6 = 0,
        float param7 = 0,
        byte confirmation = 0,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        SendLongAsync(
            new CommandLong(
                Param1: param1,
                Param2: param2,
                Param3: param3,
                Param4: param4,
                Param5: param5,
                Param6: param6,
                Param7: param7,
                Command: command,
                TargetSystem: TargetSystem,
                TargetComponent: TargetComponent,
                Confirmation: confirmation),
            timeout,
            cancel);

    public Task<CommandAck> RequestMessageAsync(
        int messageId,
        float param2 = 0,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_REQUEST_MESSAGE,
            param1: messageId,
            param2: param2,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> SetMessageIntervalAsync(
        int messageId,
        int intervalUs,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
            param1: messageId,
            param2: intervalUs,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> StopMessageIntervalAsync(
        int messageId,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        SetMessageIntervalAsync(messageId, 0, timeout, cancel);

    public Task<CommandAck> SetMissionCurrentAsync(
        int sequence,
        bool resetMission = false,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_DO_SET_MISSION_CURRENT,
            param1: sequence,
            param2: resetMission ? 1 : 0,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> ArmAsync(
        bool force = false,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
            param1: 1,
            param2: force ? 21196 : 0,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> DisarmAsync(
        bool force = false,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
            param1: 0,
            param2: force ? 21196 : 0,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> TakeoffAsync(
        double altitude = 10,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(
            MavCmd.MAV_CMD_NAV_TAKEOFF,
            param7: (float)altitude,
            timeout: timeout,
            cancel: cancel);

    public Task<CommandAck> LandAsync(
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(MavCmd.MAV_CMD_NAV_LAND, timeout: timeout, cancel: cancel);

    public Task<CommandAck> ReturnToLaunchAsync(
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null) =>
        CommandLongAsync(MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH, timeout: timeout, cancel: cancel);

    public async Task<CommandAck> WaitForAckAsync(
        MavCmd command,
        TimeSpan? timeout = null,
        MavlinkCancellationToken? cancel = null)
    {
        var message = await _session.WaitForMessageAsync(
            message => message is CommandAck ack && ack.Command == command,
            fromSystemId: TargetSystem,
            timeout: timeout ?? DefaultTimeout,
            cancel: cancel).ConfigureAwait(false);

        return (CommandAck)message;
    }
}

/// <summary>Vehicle-side command handler for embedding in autopilot code.</summary>
public sealed class CommandServer : IAsyncDisposable
{
    private readonly MavlinkSession _session;
    private readonly Func<CommandLong, Task<MavResult>>? _onCommandLong;
    private readonly Func<CommandInt, Task<MavResult>>? _onCommandInt;
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _frameTask;

    public CommandServer(
        MavlinkSession session,
        Func<CommandLong, Task<MavResult>>? onCommandLong = null,
        Func<CommandInt, Task<MavResult>>? onCommandInt = null)
    {
        _session = session;
        _onCommandLong = onCommandLong;
        _onCommandInt = onCommandInt;
        _frameTask = Task.Run(ProcessFramesAsync);
    }

    public async ValueTask DisposeAsync() => await CloseAsync().ConfigureAwait(false);

    public async Task CloseAsync()
    {
        _cts.Cancel();
        try
        {
            await _frameTask.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }

        _cts.Dispose();
    }

    private async Task ProcessFramesAsync()
    {
        await foreach (var frame in _session.Frames.WithCancellation(_cts.Token).ConfigureAwait(false))
        {
            await OnFrameAsync(frame).ConfigureAwait(false);
        }
    }

    private async Task OnFrameAsync(MavlinkFrame frame)
    {
        switch (frame.Message)
        {
            case CommandLong commandLong:
                if (commandLong.TargetSystem != _session.SystemId)
                {
                    return;
                }

                var longResult = _onCommandLong is not null
                    ? await _onCommandLong(commandLong).ConfigureAwait(false)
                    : MavResult.MAV_RESULT_ACCEPTED;
                await SendAckAsync(frame, commandLong.Command, longResult).ConfigureAwait(false);
                break;

            case CommandInt commandInt:
                if (commandInt.TargetSystem != _session.SystemId)
                {
                    return;
                }

                var intResult = _onCommandInt is not null
                    ? await _onCommandInt(commandInt).ConfigureAwait(false)
                    : MavResult.MAV_RESULT_ACCEPTED;
                await SendAckAsync(frame, commandInt.Command, intResult).ConfigureAwait(false);
                break;
        }
    }

    private Task SendAckAsync(MavlinkFrame requestFrame, MavCmd command, MavResult result) =>
        _session.SendAsync(
            new CommandAck(
                Command: command,
                Result: result,
                Progress: 0,
                ResultParam2: 0,
                TargetSystem: requestFrame.SystemId,
                TargetComponent: requestFrame.ComponentId),
            CancellationToken.None);
}
