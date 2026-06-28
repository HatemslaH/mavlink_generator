import {
  CommandAck,
  MavCmd,
  MavComponent,
  MavFrame,
  MavMissionResult,
  MavMissionType,
  MavResult,
  MissionAck,
  MissionClearAll,
  MissionCount,
  MissionItem,
  MissionItemInt,
  MissionRequest,
  MissionRequestInt,
  MissionRequestList,
  MissionSetCurrent,
  type CommandInt,
  type CommandLong,
} from '../mavlink';
import type { MavlinkFrame } from '../mavlink_frame';
import { MavlinkMessage } from '../mavlink_message';
import type { CommandProtocol } from './command_protocol';
import { MavlinkCancellationToken } from './mavlink_cancellation';
import { MavlinkSession } from './mavlink_session';

/** Helpers for building and converting mission plan items. */
export class MissionItems {
  private constructor() {}

  /** Build a global waypoint using scaled integer lat/lon (MAVLink convention). */
  static waypoint(options: {
    seq: number;
    latitude: number;
    longitude: number;
    altitude: number;
    targetSystem: number;
    targetComponent: number;
    command?: MavCmd;
    frame?: MavFrame;
    missionType?: MavMissionType;
    param1?: number;
    param2?: number;
    param3?: number;
    param4?: number;
    current?: number;
    autocontinue?: number;
  }): MissionItemInt {
    return new MissionItemInt(
      options.param1 ?? 0,
      options.param2 ?? 0,
      options.param3 ?? 0,
      options.param4 ?? 0,
      Math.round(options.latitude * 1e7),
      Math.round(options.longitude * 1e7),
      options.altitude,
      options.seq,
      options.command ?? MavCmd.MAV_CMD_NAV_WAYPOINT,
      options.targetSystem,
      options.targetComponent,
      options.frame ?? MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
      options.current ?? 0,
      options.autocontinue ?? 1,
      options.missionType ?? MavMissionType.MAV_MISSION_TYPE_MISSION,
    );
  }

  /** Convert a [MissionItemInt] to the legacy [MissionItem] representation. */
  static toLegacyItem(item: MissionItemInt): MissionItem {
    return new MissionItem(
      item.param1,
      item.param2,
      item.param3,
      item.param4,
      item.x / 1e7,
      item.y / 1e7,
      item.z,
      item.seq,
      item.command,
      item.targetSystem,
      item.targetComponent,
      item.frame,
      item.current,
      item.autocontinue,
      item.missionType,
    );
  }

  /** Convert a legacy [MissionItem] to [MissionItemInt]. */
  static fromLegacyItem(item: MissionItem): MissionItemInt {
    return new MissionItemInt(
      item.param1,
      item.param2,
      item.param3,
      item.param4,
      Math.round(item.x * 1e7),
      Math.round(item.y * 1e7),
      item.z,
      item.seq,
      item.command,
      item.targetSystem,
      item.targetComponent,
      item.frame,
      item.current,
      item.autocontinue,
      item.missionType,
    );
  }

  /** Re-number items sequentially starting from zero. */
  static withSequentialSeq(items: MissionItemInt[]): MissionItemInt[] {
    return items.map((item, index) =>
      new MissionItemInt(
        item.param1,
        item.param2,
        item.param3,
        item.param4,
        item.x,
        item.y,
        item.z,
        index,
        item.command,
        item.targetSystem,
        item.targetComponent,
        item.frame,
        item.current,
        item.autocontinue,
        item.missionType,
      ),
    );
  }
}

export type MissionUploadProgressCallback = (
  sent: number,
  total: number,
  item: MissionItemInt,
) => void;

export type MissionDownloadProgressCallback = (
  received: number,
  total: number,
  item: MissionItemInt,
) => void;

/** Result of [MissionProtocol.setCurrentWithCommand]. */
export interface MissionSetCurrentResult {
  readonly sequence: number;
  readonly commandAck?: CommandAck;
}

/** GCS-side MAVLink mission protocol client. */
export class MissionProtocol {
  readonly session: MavlinkSession;
  readonly targetSystem: number;
  readonly targetComponent: number;
  readonly itemTimeoutMs: number;
  readonly operationTimeoutMs: number;

  constructor(options: {
    session: MavlinkSession;
    targetSystem: number;
    targetComponent: number;
    itemTimeoutMs?: number;
    operationTimeoutMs?: number;
  }) {
    this.session = options.session;
    this.targetSystem = options.targetSystem;
    this.targetComponent = options.targetComponent;
    this.itemTimeoutMs = options.itemTimeoutMs ?? 3000;
    this.operationTimeoutMs = options.operationTimeoutMs ?? 10_000;
  }

  /** Upload a mission plan to the vehicle. */
  async upload(
    items: MissionItemInt[],
    options: {
      missionType?: MavMissionType;
      onProgress?: MissionUploadProgressCallback;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<MavMissionResult> {
    const missionType =
      options.missionType ?? MavMissionType.MAV_MISSION_TYPE_MISSION;
    options.cancel?.throwIfCancelled();
    const plan = MissionItems.withSequentialSeq(items);

    await this.session.send(
      new MissionCount(
        plan.length,
        this.targetSystem,
        this.targetComponent,
        missionType,
      ),
    );

    for (const item of plan) {
      options.cancel?.throwIfCancelled();

      const request = await this.session.waitForMessage({
        predicate: (message) =>
          MissionProtocol.isItemRequest(message, item.seq, missionType),
        fromSystemId: this.targetSystem,
        timeoutMs: this.itemTimeoutMs,
        cancel: options.cancel,
      });

      if (MavlinkMessage.isMessageOf<MissionRequestInt>(request, MissionRequestInt)) {
        await this.session.send(item);
      } else if (MavlinkMessage.isMessageOf<MissionRequest>(request, MissionRequest)) {
        await this.session.send(MissionItems.toLegacyItem(item));
      }

      options.onProgress?.call(undefined, item.seq + 1, plan.length, item);
    }

    const ack = await this.waitForMissionAck(options.cancel);

    return ack.type;
  }

  /** Download a mission plan from the vehicle. */
  async download(
    options: {
      missionType?: MavMissionType;
      onProgress?: MissionDownloadProgressCallback;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<MissionItemInt[]> {
    const missionType =
      options.missionType ?? MavMissionType.MAV_MISSION_TYPE_MISSION;
    options.cancel?.throwIfCancelled();

    await this.session.send(
      new MissionRequestList(
        this.targetSystem,
        this.targetComponent,
        missionType,
      ),
    );

    const countMessage = await this.session.waitForMessageType(MissionCount, {
      fromSystemId: this.targetSystem,
      timeoutMs: this.operationTimeoutMs,
      cancel: options.cancel,
    });

    const items: MissionItemInt[] = [];

    for (let seq = 0; seq < countMessage.count; seq++) {
      options.cancel?.throwIfCancelled();

      await this.session.send(
        new MissionRequestInt(
          seq,
          this.targetSystem,
          this.targetComponent,
          missionType,
        ),
      );

      const itemMessage = await this.session.waitForMessage({
        predicate: (message) => {
          if (MavlinkMessage.isMessageOf<MissionItemInt>(message, MissionItemInt)) {
            return (
              message.seq === seq && message.missionType === missionType
            );
          }
          if (MavlinkMessage.isMessageOf<MissionItem>(message, MissionItem)) {
            return (
              message.seq === seq && message.missionType === missionType
            );
          }
          return false;
        },
        fromSystemId: this.targetSystem,
        timeoutMs: this.itemTimeoutMs,
        cancel: options.cancel,
      });

      const item = MavlinkMessage.isMessageOf<MissionItemInt>(
        itemMessage,
        MissionItemInt,
      )
        ? itemMessage
        : MissionItems.fromLegacyItem(itemMessage as MissionItem);

      items.push(item);
      options.onProgress?.call(undefined, items.length, countMessage.count, item);
    }

    await this.session.send(
      new MissionAck(
        this.targetSystem,
        this.targetComponent,
        MavMissionResult.MAV_MISSION_ACCEPTED,
        missionType,
      ),
    );

    return items;
  }

  /** Clear all mission items of the given type on the vehicle. */
  async clear(
    options: {
      missionType?: MavMissionType;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<MavMissionResult> {
    const missionType =
      options.missionType ?? MavMissionType.MAV_MISSION_TYPE_MISSION;
    await this.session.send(
      new MissionClearAll(
        this.targetSystem,
        this.targetComponent,
        missionType,
      ),
    );

    const ack = await this.waitForMissionAck(options.cancel);

    return ack.type;
  }

  /** Set the active mission item sequence number on the vehicle. */
  async setCurrent(
    seq: number,
    cancel?: MavlinkCancellationToken,
  ): Promise<void> {
    cancel?.throwIfCancelled();
    await this.session.send(
      new MissionSetCurrent(seq, this.targetSystem, this.targetComponent),
    );
  }

  /** Send [MissionSetCurrent] and optionally [CommandProtocol.setMissionCurrent]. */
  async setCurrentWithCommand(
    seq: number,
    options: {
      command?: CommandProtocol;
      alsoSendCommand?: boolean;
      resetMission?: boolean;
      cancel?: MavlinkCancellationToken;
    } = {},
  ): Promise<MissionSetCurrentResult> {
    options.cancel?.throwIfCancelled();
    await this.setCurrent(seq, options.cancel);

    let commandAck: CommandAck | undefined;
    if (
      (options.alsoSendCommand ?? true) &&
      options.command !== undefined
    ) {
      commandAck = await options.command.setMissionCurrent(seq, {
        resetMission: options.resetMission,
        cancel: options.cancel,
      });
    }

    return { sequence: seq, commandAck };
  }

  private async waitForMissionAck(
    cancel?: MavlinkCancellationToken,
  ): Promise<MissionAck> {
    return this.session.waitForMessage({
      predicate: (message) => {
        if (!MavlinkMessage.isMessageOf<MissionAck>(message, MissionAck)) {
          return false;
        }
        return (
          message.targetSystem === this.session.systemId &&
          (message.targetComponent === this.session.componentId ||
            message.targetComponent === MavComponent.MAV_COMP_ID_ALL)
        );
      },
      fromSystemId: this.targetSystem,
      timeoutMs: this.operationTimeoutMs,
      cancel,
    }) as Promise<MissionAck>;
  }

  private static isItemRequest(
    message: MavlinkMessage,
    seq: number,
    missionType: MavMissionType,
  ): boolean {
    if (MavlinkMessage.isMessageOf<MissionRequestInt>(message, MissionRequestInt)) {
      return message.seq === seq && message.missionType === missionType;
    }
    if (MavlinkMessage.isMessageOf<MissionRequest>(message, MissionRequest)) {
      return message.seq === seq && message.missionType === missionType;
    }
    return false;
  }
}

/** Vehicle-side mission protocol handler for embedding in autopilot code. */
export class MissionServer {
  readonly session: MavlinkSession;
  readonly missionType: MavMissionType;

  private readonly _items: MissionItemInt[] = [];
  private readonly _incoming = new Map<number, MissionItemInt>();
  private _incomingCount: number | null = null;
  private readonly _unsubscribe: () => void;

  constructor(options: {
    session: MavlinkSession;
    initialMission?: MissionItemInt[];
    missionType?: MavMissionType;
  }) {
    this.session = options.session;
    this.missionType =
      options.missionType ?? MavMissionType.MAV_MISSION_TYPE_MISSION;
    if (options.initialMission !== undefined) {
      this._items.push(...options.initialMission);
    }
    const subscription = this.session.listenMessage((message, frame) => {
      void this._onFrame(frame, message);
    });
    this._unsubscribe = () => subscription.cancel();
  }

  get items(): readonly MissionItemInt[] {
    return this._items;
  }

  async close(): Promise<void> {
    this._unsubscribe();
  }

  replaceMission(items: MissionItemInt[]): void {
    this._items.length = 0;
    this._items.push(...MissionItems.withSequentialSeq(items));
    this._incoming.clear();
    this._incomingCount = null;
  }

  private async _onFrame(
    frame: MavlinkFrame,
    message: MavlinkMessage,
  ): Promise<void> {
    if (MavlinkMessage.isMessageOf<MissionCount>(message, MissionCount) && this._targetsUs(message)) {
      if (message.missionType !== this.missionType) {
        return;
      }
      this._incomingCount = message.count;
      this._incoming.clear();
      if (message.count > 0) {
        await this._requestUploadItem(frame, 0);
      } else {
        await this._sendUploadAck(frame);
      }
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionItemInt>(message, MissionItemInt) && this._targetsUs(message)) {
      if (message.missionType !== this.missionType) {
        return;
      }
      await this._storeIncomingItem(frame, message);
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionItem>(message, MissionItem) && this._targetsUs(message)) {
      if (message.missionType !== this.missionType) {
        return;
      }
      await this._storeIncomingItem(frame, MissionItems.fromLegacyItem(message));
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionRequestInt>(message, MissionRequestInt) && this._targetsUs(message)) {
      await this._sendRequestedItem(frame, message.seq);
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionRequest>(message, MissionRequest) && this._targetsUs(message)) {
      await this._sendRequestedItem(frame, message.seq);
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionRequestList>(message, MissionRequestList) && this._targetsUs(message)) {
      if (message.missionType !== this.missionType) {
        return;
      }
      await this.session.send(
        new MissionCount(
          this._items.length,
          frame.systemId,
          frame.componentId,
          this.missionType,
        ),
      );
      return;
    }

    if (MavlinkMessage.isMessageOf<MissionClearAll>(message, MissionClearAll) && this._targetsUs(message)) {
      if (message.missionType !== this.missionType) {
        return;
      }
      this._items.length = 0;
      this._incoming.clear();
      this._incomingCount = null;
      await this.session.send(
        new MissionAck(
          frame.systemId,
          frame.componentId,
          MavMissionResult.MAV_MISSION_ACCEPTED,
          this.missionType,
        ),
      );
    }
  }

  private async _storeIncomingItem(
    frame: MavlinkFrame,
    item: MissionItemInt,
  ): Promise<void> {
    this._incoming.set(item.seq, item);
    const expected = this._incomingCount;
    if (expected === null) {
      return;
    }

    if (this._incoming.size < expected) {
      await this._requestUploadItem(frame, item.seq + 1);
      return;
    }

    this._items.length = 0;
    for (let index = 0; index < expected; index++) {
      const stored = this._incoming.get(index);
      if (stored !== undefined) {
        this._items.push(stored);
      }
    }
    this._incoming.clear();
    this._incomingCount = null;
    await this._sendUploadAck(frame);
  }

  private async _requestUploadItem(
    requestFrame: MavlinkFrame,
    seq: number,
  ): Promise<void> {
    await this.session.send(
      new MissionRequestInt(
        seq,
        requestFrame.systemId,
        requestFrame.componentId,
        this.missionType,
      ),
    );
  }

  private async _sendUploadAck(requestFrame: MavlinkFrame): Promise<void> {
    await this.session.send(
      new MissionAck(
        requestFrame.systemId,
        requestFrame.componentId,
        MavMissionResult.MAV_MISSION_ACCEPTED,
        this.missionType,
      ),
    );
  }

  private async _sendRequestedItem(
    requestFrame: MavlinkFrame,
    seq: number,
  ): Promise<void> {
    if (seq < 0 || seq >= this._items.length) {
      await this.session.send(
        new MissionAck(
          requestFrame.systemId,
          requestFrame.componentId,
          MavMissionResult.MAV_MISSION_INVALID_SEQUENCE,
          this.missionType,
        ),
      );
      return;
    }

    await this.session.send(this._items[seq]!);
  }

  private _targetsUs(message: MavlinkMessage): boolean {
    const target = MissionServer.readTarget(message);
    if (target === null) {
      return false;
    }
    return this._matchesTarget(target.system, target.component);
  }

  private static readTarget(
    message: MavlinkMessage,
  ): { system: number; component: number } | null {
    if (
      MavlinkMessage.isMessageOf(message, MissionCount) ||
      MavlinkMessage.isMessageOf(message, MissionItemInt) ||
      MavlinkMessage.isMessageOf(message, MissionItem) ||
      MavlinkMessage.isMessageOf(message, MissionRequestInt) ||
      MavlinkMessage.isMessageOf(message, MissionRequest) ||
      MavlinkMessage.isMessageOf(message, MissionRequestList) ||
      MavlinkMessage.isMessageOf(message, MissionClearAll)
    ) {
      const targeted = message as MissionCount;
      return {
        system: targeted.targetSystem,
        component: targeted.targetComponent,
      };
    }
    return null;
  }

  private _matchesTarget(targetSystem: number, targetComponent: number): boolean {
    if (targetSystem !== this.session.systemId && targetSystem !== 0) {
      return false;
    }
    if (
      targetComponent !== this.session.componentId &&
      targetComponent !== MavComponent.MAV_COMP_ID_ALL
    ) {
      return false;
    }
    return true;
  }
}
