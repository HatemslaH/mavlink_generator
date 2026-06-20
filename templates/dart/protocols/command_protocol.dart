import 'dart:async';

import '../mavlink.dart';
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
  Future<CommandAck> sendLong(CommandLong command, {Duration? timeout}) async {
    await session.send(command);
    return waitForAck(command.command, timeout: timeout);
  }

  /// Send [CommandInt] and wait for [CommandAck].
  Future<CommandAck> sendInt(CommandInt command, {Duration? timeout}) async {
    await session.send(command);
    return waitForAck(command.command, timeout: timeout);
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
    );
  }

  /// Request a single message using MAV_CMD_REQUEST_MESSAGE.
  Future<CommandAck> requestMessage(int messageId, {float param2 = 0, Duration? timeout}) {
    return commandLong(
      command: MavCmd.mavCmdRequestMessage,
      param1: messageId.toDouble(),
      param2: param2,
      timeout: timeout,
    );
  }

  /// Stream a message at the given interval using MAV_CMD_SET_MESSAGE_INTERVAL.
  ///
  /// [intervalUs] is the period in microseconds (100_000 = 10 Hz).
  Future<CommandAck> setMessageInterval(int messageId, int intervalUs, {Duration? timeout}) {
    return commandLong(
      command: MavCmd.mavCmdSetMessageInterval,
      param1: messageId.toDouble(),
      param2: intervalUs.toDouble(),
      timeout: timeout,
    );
  }

  /// Set the active mission item via MAV_CMD_DO_SET_MISSION_CURRENT.
  Future<CommandAck> setMissionCurrent(int sequence, {bool resetMission = false, Duration? timeout}) {
    return commandLong(
      command: MavCmd.mavCmdDoSetMissionCurrent,
      param1: sequence.toDouble(),
      param2: resetMission ? 1 : 0,
      timeout: timeout,
    );
  }

  Future<CommandAck> waitForAck(MavCmd command, {Duration? timeout}) {
    return session
        .waitForMessage(
          predicate: (message) => message is CommandAck && message.command == command,
          fromSystemId: targetSystem,
          timeout: timeout ?? defaultTimeout,
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
