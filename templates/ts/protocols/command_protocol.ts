import {
  CommandAck,
  CommandInt,
  CommandLong,
  MavCmd,
  MavResult,
} from '../mavlink';
import type { MavlinkFrame } from '../mavlink_frame';
import { MavlinkCancellationToken } from './mavlink_cancellation';
import { MavlinkSession } from './mavlink_session';

/** GCS-side MAVLink command protocol client. */
export class CommandProtocol {
  readonly session: MavlinkSession;
  readonly targetSystem: number;
  readonly targetComponent: number;
  readonly defaultTimeoutMs: number;

  constructor(options: {
    session: MavlinkSession;
    targetSystem: number;
    targetComponent: number;
    defaultTimeoutMs?: number;
  }) {
    this.session = options.session;
    this.targetSystem = options.targetSystem;
    this.targetComponent = options.targetComponent;
    this.defaultTimeoutMs = options.defaultTimeoutMs ?? 5000;
  }

  /** Send [CommandLong] and wait for [CommandAck]. */
  async sendLong(
    command: CommandLong,
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    await this.session.send(command);
    return this.waitForAck(command.command, options);
  }

  /** Send [CommandInt] and wait for [CommandAck]. */
  async sendInt(
    command: CommandInt,
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    await this.session.send(command);
    return this.waitForAck(command.command, options);
  }

  /** Build and send COMMAND_LONG, waiting for COMMAND_ACK. */
  commandLong(options: {
    command: MavCmd;
    param1?: number;
    param2?: number;
    param3?: number;
    param4?: number;
    param5?: number;
    param6?: number;
    param7?: number;
    confirmation?: number;
    timeoutMs?: number;
    cancel?: MavlinkCancellationToken;
  }): Promise<CommandAck> {
    return this.sendLong(
      new CommandLong(
        options.param1 ?? 0,
        options.param2 ?? 0,
        options.param3 ?? 0,
        options.param4 ?? 0,
        options.param5 ?? 0,
        options.param6 ?? 0,
        options.param7 ?? 0,
        options.command,
        this.targetSystem,
        this.targetComponent,
        options.confirmation ?? 0,
      ),
      { timeoutMs: options.timeoutMs, cancel: options.cancel },
    );
  }

  /** Request a single message using MAV_CMD_REQUEST_MESSAGE. */
  requestMessage(
    messageId: number,
    options: {
      param2?: number;
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_REQUEST_MESSAGE,
      param1: messageId,
      param2: options.param2 ?? 0,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Stream a message at the given interval using MAV_CMD_SET_MESSAGE_INTERVAL. */
  setMessageInterval(
    messageId: number,
    intervalUs: number,
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_SET_MESSAGE_INTERVAL,
      param1: messageId,
      param2: intervalUs,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Stop streaming a message (interval 0). */
  stopMessageInterval(
    messageId: number,
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.setMessageInterval(messageId, 0, options);
  }

  /** Set the active mission item via MAV_CMD_DO_SET_MISSION_CURRENT. */
  setMissionCurrent(
    sequence: number,
    options: {
      resetMission?: boolean;
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_DO_SET_MISSION_CURRENT,
      param1: sequence,
      param2: options.resetMission ? 1 : 0,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Arm motors via MAV_CMD_COMPONENT_ARM_DISARM (param1 = 1). */
  arm(
    options: {
      force?: boolean;
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
      param1: 1,
      param2: options.force ? 21196 : 0,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Disarm motors via MAV_CMD_COMPONENT_ARM_DISARM (param1 = 0). */
  disarm(
    options: {
      force?: boolean;
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_COMPONENT_ARM_DISARM,
      param1: 0,
      param2: options.force ? 21196 : 0,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Takeoff to [altitude] metres via MAV_CMD_NAV_TAKEOFF. */
  takeoff(
    options: {
      altitude?: number;
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_TAKEOFF,
      param7: options.altitude ?? 10,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Land in place via MAV_CMD_NAV_LAND. */
  land(
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_LAND,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  /** Return to launch via MAV_CMD_NAV_RETURN_TO_LAUNCH. */
  returnToLaunch(
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    return this.commandLong({
      command: MavCmd.MAV_CMD_NAV_RETURN_TO_LAUNCH,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
  }

  async waitForAck(
    command: MavCmd,
    options: {
      timeoutMs?: number;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<CommandAck> {
    const message = await this.session.waitForMessage({
      predicate: (message) =>
        message instanceof CommandAck && message.command === command,
      fromSystemId: this.targetSystem,
      timeoutMs: options.timeoutMs ?? this.defaultTimeoutMs,
      cancel: options.cancel,
    });
    return message as CommandAck;
  }
}

/** Vehicle-side command handler for embedding in autopilot code. */
export class CommandServer {
  readonly session: MavlinkSession;
  readonly onCommandLong?:
    | ((command: CommandLong) => Promise<MavResult> | MavResult)
    | undefined;
  readonly onCommandInt?:
    | ((command: CommandInt) => Promise<MavResult> | MavResult)
    | undefined;

  private readonly _unsubscribe: () => void;

  constructor(options: {
    session: MavlinkSession;
    onCommandLong?: (command: CommandLong) => Promise<MavResult> | MavResult;
    onCommandInt?: (command: CommandInt) => Promise<MavResult> | MavResult;
  }) {
    this.session = options.session;
    this.onCommandLong = options.onCommandLong;
    this.onCommandInt = options.onCommandInt;
    const subscription = this.session.listenMessage((message, frame) => {
      void this._onFrame(frame, message);
    });
    this._unsubscribe = () => subscription.cancel();
  }

  async close(): Promise<void> {
    this._unsubscribe();
  }

  private async _onFrame(
    frame: MavlinkFrame,
    message: import('../mavlink_message').MavlinkMessage,
  ): Promise<void> {
    if (!(message instanceof CommandLong) && !(message instanceof CommandInt)) {
      return;
    }
    if (message instanceof CommandLong) {
      if (message.targetSystem !== this.session.systemId) {
        return;
      }
      const result =
        (await this.onCommandLong?.call(undefined, message)) ??
        MavResult.MAV_RESULT_ACCEPTED;
      await this._sendAck(frame, message.command, result);
      return;
    }

    if (message instanceof CommandInt) {
      if (message.targetSystem !== this.session.systemId) {
        return;
      }
      const result =
        (await this.onCommandInt?.call(undefined, message)) ??
        MavResult.MAV_RESULT_ACCEPTED;
      await this._sendAck(frame, message.command, result);
    }
  }

  private async _sendAck(
    requestFrame: MavlinkFrame,
    command: MavCmd,
    result: MavResult,
  ): Promise<void> {
    await this.session.send(
      new CommandAck(
        command,
        result,
        0,
        0,
        requestFrame.systemId,
        requestFrame.componentId,
      ),
    );
  }
}
