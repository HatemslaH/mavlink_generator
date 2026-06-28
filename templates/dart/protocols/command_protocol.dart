import 'dart:async';

import '../mavlink.dart';
import 'mavlink_cancellation.dart';
import 'mavlink_session.dart';

/// GCS-side MAVLink command protocol client.
///
/// Implements COMMAND_LONG / COMMAND_INT flows and common helpers per
/// https://mavlink.io/en/services/command.html
class CommandProtocol {
  CommandProtocol({
    required this.session,
    required this.targetSystem,
    required this.targetComponent,
    this.defaultTimeout = const Duration(seconds: 5),
  });

  final MavlinkSession session;
  final int targetSystem;
  final int targetComponent;
  final Duration defaultTimeout;

  /// Send [CommandLong] and wait for [CommandAck].
  Future<CommandAck> sendLong(CommandLong command, {Duration? timeout, MavlinkCancellationToken? cancel}) async {
    await session.send(command);
    return waitForAck(command.command, timeout: timeout, cancel: cancel);
  }

  /// Send [CommandInt] and wait for [CommandAck].
  Future<CommandAck> sendInt(CommandInt command, {Duration? timeout, MavlinkCancellationToken? cancel}) async {
    await session.send(command);
    return waitForAck(command.command, timeout: timeout, cancel: cancel);
  }

  /// Build and send COMMAND_LONG, waiting for COMMAND_ACK.
  Future<CommandAck> commandLong({
    required MavCmd command,
    float param1 = 0,
    float param2 = 0,
    float param3 = 0,
    float param4 = 0,
    float param5 = 0,
    float param6 = 0,
    float param7 = 0,
    uint8_t confirmation = 0,
    Duration? timeout,
    MavlinkCancellationToken? cancel,
  }) {
    return sendLong(
      CommandLong(
        param1: param1,
        param2: param2,
        param3: param3,
        param4: param4,
        param5: param5,
        param6: param6,
        param7: param7,
        command: command,
        targetSystem: targetSystem,
        targetComponent: targetComponent,
        confirmation: confirmation,
      ),
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Request a single message using MAV_CMD_REQUEST_MESSAGE.
  Future<CommandAck> requestMessage(int messageId, {float param2 = 0, Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdRequestMessage,
      param1: messageId.toDouble(),
      param2: param2,
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Stream a message at the given interval using MAV_CMD_SET_MESSAGE_INTERVAL.
  ///
  /// [intervalUs] is the period in microseconds (100_000 = 10 Hz).
  Future<CommandAck> setMessageInterval(int messageId, int intervalUs, {Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdSetMessageInterval,
      param1: messageId.toDouble(),
      param2: intervalUs.toDouble(),
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Stop streaming a message (interval 0).
  Future<CommandAck> stopMessageInterval(int messageId, {Duration? timeout, MavlinkCancellationToken? cancel}) {
    return setMessageInterval(messageId, 0, timeout: timeout, cancel: cancel);
  }

  /// Set the active mission item via MAV_CMD_DO_SET_MISSION_CURRENT.
  Future<CommandAck> setMissionCurrent(int sequence, {bool resetMission = false, Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdDoSetMissionCurrent,
      param1: sequence.toDouble(),
      param2: resetMission ? 1 : 0,
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Arm motors via MAV_CMD_COMPONENT_ARM_DISARM (param1 = 1).
  Future<CommandAck> arm({bool force = false, Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdComponentArmDisarm,
      param1: 1,
      param2: force ? 21196 : 0,
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Disarm motors via MAV_CMD_COMPONENT_ARM_DISARM (param1 = 0).
  Future<CommandAck> disarm({bool force = false, Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdComponentArmDisarm,
      param1: 0,
      param2: force ? 21196 : 0,
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Takeoff to [altitude] metres via MAV_CMD_NAV_TAKEOFF.
  Future<CommandAck> takeoff({double altitude = 10, Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(
      command: MavCmd.mavCmdNavTakeoff,
      param7: altitude,
      timeout: timeout,
      cancel: cancel,
    );
  }

  /// Land in place via MAV_CMD_NAV_LAND.
  Future<CommandAck> land({Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(command: MavCmd.mavCmdNavLand, timeout: timeout, cancel: cancel);
  }

  /// Return to launch via MAV_CMD_NAV_RETURN_TO_LAUNCH.
  Future<CommandAck> returnToLaunch({Duration? timeout, MavlinkCancellationToken? cancel}) {
    return commandLong(command: MavCmd.mavCmdNavReturnToLaunch, timeout: timeout, cancel: cancel);
  }

  Future<CommandAck> waitForAck(MavCmd command, {Duration? timeout, MavlinkCancellationToken? cancel}) {
    return session
        .waitForMessage(
          predicate: (message) => message is CommandAck && message.command == command,
          fromSystemId: targetSystem,
          timeout: timeout ?? defaultTimeout,
          cancel: cancel,
        )
        .then((message) => message as CommandAck);
  }
}

/// Vehicle-side command handler for embedding in autopilot code.
class CommandServer {
  CommandServer({required this.session, this.onCommandLong, this.onCommandInt}) {
    _subscription = session.frames.listen(_onFrame);
  }

  final MavlinkSession session;
  final Future<MavResult> Function(CommandLong command)? onCommandLong;
  final Future<MavResult> Function(CommandInt command)? onCommandInt;
  late final StreamSubscription<MavlinkFrame> _subscription;

  Future<void> close() async {
    await _subscription.cancel();
  }

  Future<void> _onFrame(MavlinkFrame frame) async {
    final message = frame.message;

    if (message is CommandLong) {
      if (message.targetSystem != session.systemId) {
        return;
      }
      final result = await onCommandLong?.call(message) ?? MavResult.mavResultAccepted;
      await _sendAck(frame, message.command, result);
      return;
    }

    if (message is CommandInt) {
      if (message.targetSystem != session.systemId) {
        return;
      }
      final result = await onCommandInt?.call(message) ?? MavResult.mavResultAccepted;
      await _sendAck(frame, message.command, result);
    }
  }

  Future<void> _sendAck(MavlinkFrame requestFrame, MavCmd command, MavResult result) async {
    await session.send(
      CommandAck(
        command: command,
        result: result,
        progress: 0,
        resultParam2: 0,
        targetSystem: requestFrame.systemId,
        targetComponent: requestFrame.componentId,
      ),
    );
  }
}
