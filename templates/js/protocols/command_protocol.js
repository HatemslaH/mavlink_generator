import {
  CommandAck,
  CommandInt,
  CommandLong,
  MavCmd,
  MavResult,
} from '../mavlink.js';
import { MavlinkMessage } from '../mavlink_message.js';

/** GCS-side MAVLink command protocol client. */
export class CommandProtocol {
  constructor({ session, targetSystem, targetComponent, defaultTimeoutMs = 5000 }) {
    this.session = session;
    this.targetSystem = targetSystem;
    this.targetComponent = targetComponent;
    this.defaultTimeoutMs = defaultTimeoutMs;
  }

  async sendLong(command, { timeoutMs = null, cancel = null } = {}) {
    await this.session.send(command);
    return this.waitForAck(command.command, { timeoutMs, cancel });
  }

  async sendInt(command, { timeoutMs = null, cancel = null } = {}) {
    await this.session.send(command);
    return this.waitForAck(command.command, { timeoutMs, cancel });
  }

  commandLong({
    command,
    param1 = 0,
    param2 = 0,
    param3 = 0,
    param4 = 0,
    param5 = 0,
    param6 = 0,
    param7 = 0,
    confirmation = 0,
    timeoutMs = null,
    cancel = null,
  }) {
    return this.sendLong(
      new CommandLong(
        param1,
        param2,
        param3,
        param4,
        param5,
        param6,
        param7,
        command,
        this.targetSystem,
        this.targetComponent,
        confirmation,
      ),
      { timeoutMs, cancel },
    );
  }

  requestMessage(messageId, { param2 = 0, timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_REQUEST_MESSAGE,
      param1: messageId,
      param2,
      timeoutMs,
      cancel,
    });
  }

  setMessageInterval(messageId, intervalUs, { timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
      param1: messageId,
      param2: intervalUs,
      timeoutMs,
      cancel,
    });
  }

  stopMessageInterval(messageId, { timeoutMs = null, cancel = null } = {}) {
    return this.setMessageInterval(messageId, 0, { timeoutMs, cancel });
  }

  setMissionCurrent(sequence, { resetMission = false, timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_DO_SET_MISSION_CURRENT,
      param1: sequence,
      param2: resetMission ? 1 : 0,
      timeoutMs,
      cancel,
    });
  }

  arm({ force = false, timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
      param1: 1,
      param2: force ? 21196 : 0,
      timeoutMs,
      cancel,
    });
  }

  disarm({ force = false, timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
      param1: 0,
      param2: force ? 21196 : 0,
      timeoutMs,
      cancel,
    });
  }

  takeoff({ altitude = 10, timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_TAKEOFF,
      param7: altitude,
      timeoutMs,
      cancel,
    });
  }

  land({ timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_LAND,
      timeoutMs,
      cancel,
    });
  }

  returnToLaunch({ timeoutMs = null, cancel = null } = {}) {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
      timeoutMs,
      cancel,
    });
  }

  async waitForAck(command, { timeoutMs = null, cancel = null } = {}) {
    return this.session.waitForMessage({
      predicate: (message) =>
        MavlinkMessage.isMessageOf(message, CommandAck) && message.command === command,
      fromSystemId: this.targetSystem,
      fromComponentId: this.targetComponent,
      timeoutMs: timeoutMs ?? this.defaultTimeoutMs,
      cancel,
    });
  }
}

/** Vehicle-side command handler for embedding in autopilot code. */
export class CommandServer {
  constructor({ session, onCommandLong = null, onCommandInt = null }) {
    this.session = session;
    this.onCommandLong = onCommandLong;
    this.onCommandInt = onCommandInt;
    const subscription = this.session.listenMessage((message, frame) => {
      void this._onFrame(frame, message);
    });
    this._unsubscribe = () => subscription.cancel();
  }

  async close() {
    this._unsubscribe();
  }

  async _onFrame(frame, message) {
    if (
      !MavlinkMessage.isMessageOf(message, CommandLong) &&
      !MavlinkMessage.isMessageOf(message, CommandInt)
    ) {
      return;
    }

    if (MavlinkMessage.isMessageOf(message, CommandLong)) {
      if (message.targetSystem !== this.session.systemId) {
        return;
      }
      const result = (await this.onCommandLong?.(message)) ?? MavResult.MAV_RESULT_ACCEPTED;
      await this._sendAck(frame, message.command, result);
      return;
    }

    if (MavlinkMessage.isMessageOf(message, CommandInt)) {
      if (message.targetSystem !== this.session.systemId) {
        return;
      }
      const result = (await this.onCommandInt?.(message)) ?? MavResult.MAV_RESULT_ACCEPTED;
      await this._sendAck(frame, message.command, result);
    }
  }

  async _sendAck(requestFrame, command, result) {
    await this.session.send(
      new CommandAck(command, result, 0, 0, requestFrame.systemId, requestFrame.componentId),
    );
  }
}
