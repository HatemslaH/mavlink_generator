import {
  MavCmd,
  MavComponent,
  MavFrame,
  MavMissionResult,
  MavMissionType,
  MissionAck,
  MissionClearAll,
  MissionCount,
  MissionItem,
  MissionItemInt,
  MissionRequest,
  MissionRequestInt,
  MissionRequestList,
  MissionSetCurrent,
} from '../mavlink.js';

/** Helpers for building and converting mission plan items. */
export class MissionItems {
  static waypoint({
    seq,
    latitude,
    longitude,
    altitude,
    targetSystem,
    targetComponent,
    command = MavCmd.MAV_CMD_NAV_WAYPOINT,
    frame = MavFrame.MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
    missionType = MavMissionType.MAV_MISSION_TYPE_MISSION,
    param1 = 0,
    param2 = 0,
    param3 = 0,
    param4 = 0,
    current = 0,
    autocontinue = 1,
  }) {
    return new MissionItemInt(
      param1,
      param2,
      param3,
      param4,
      Math.round(latitude * 1e7),
      Math.round(longitude * 1e7),
      altitude,
      seq,
      command,
      targetSystem,
      targetComponent,
      frame,
      current,
      autocontinue,
      missionType,
    );
  }

  static toLegacyItem(item) {
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
      item.target_system,
      item.target_component,
      item.frame,
      item.current,
      item.autocontinue,
      item.mission_type,
    );
  }

  static fromLegacyItem(item) {
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
      item.target_system,
      item.target_component,
      item.frame,
      item.current,
      item.autocontinue,
      item.mission_type,
    );
  }

  static withSequentialSeq(items) {
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
        item.target_system,
        item.target_component,
        item.frame,
        item.current,
        item.autocontinue,
        item.mission_type,
      ),
    );
  }
}

/** Result of [MissionProtocol.setCurrentWithCommand]. */
export class MissionSetCurrentResult {
  constructor({ sequence, commandAck = null }) {
    this.sequence = sequence;
    this.commandAck = commandAck;
  }
}

/** GCS-side MAVLink mission protocol client. */
export class MissionProtocol {
  constructor({
    session,
    targetSystem,
    targetComponent,
    itemTimeoutMs = 3000,
    operationTimeoutMs = 10000,
  }) {
    this.session = session;
    this.targetSystem = targetSystem;
    this.targetComponent = targetComponent;
    this.itemTimeoutMs = itemTimeoutMs;
    this.operationTimeoutMs = operationTimeoutMs;
  }

  async upload(items, { missionType = MavMissionType.MAV_MISSION_TYPE_MISSION, onProgress = null, cancel = null } = {}) {
    cancel?.throwIfCancelled();
    const plan = MissionItems.withSequentialSeq(items);

    await this.session.send(
      new MissionCount(plan.length, this.targetSystem, this.targetComponent, missionType),
    );

    for (const item of plan) {
      cancel?.throwIfCancelled();

      const request = await this.session.waitForMessage({
        predicate: (message) => this._isItemRequest(message, item.seq, missionType),
        fromSystemId: this.targetSystem,
        timeoutMs: this.itemTimeoutMs,
        cancel,
      });

      if (request instanceof MissionRequestInt) {
        await this.session.send(item);
      } else if (request instanceof MissionRequest) {
        await this.session.send(MissionItems.toLegacyItem(item));
      }

      onProgress?.(item.seq + 1, plan.length, item);
    }

    const ack = await this.session.waitForMessageType(MissionAck, {
      fromSystemId: this.targetSystem,
      timeoutMs: this.operationTimeoutMs,
      cancel,
    });

    return ack.type;
  }

  async download({ missionType = MavMissionType.MAV_MISSION_TYPE_MISSION, onProgress = null, cancel = null } = {}) {
    cancel?.throwIfCancelled();

    await this.session.send(
      new MissionRequestList(this.targetSystem, this.targetComponent, missionType),
    );

    const countMessage = await this.session.waitForMessageType(MissionCount, {
      fromSystemId: this.targetSystem,
      timeoutMs: this.operationTimeoutMs,
      cancel,
    });

    const items = [];

    for (let seq = 0; seq < countMessage.count; seq++) {
      cancel?.throwIfCancelled();

      await this.session.send(
        new MissionRequestInt(seq, this.targetSystem, this.targetComponent, missionType),
      );

      const itemMessage = await this.session.waitForMessage({
        predicate: (message) => {
          if (message instanceof MissionItemInt) {
            return message.seq === seq && message.mission_type === missionType;
          }
          if (message instanceof MissionItem) {
            return message.seq === seq && message.mission_type === missionType;
          }
          return false;
        },
        fromSystemId: this.targetSystem,
        timeoutMs: this.itemTimeoutMs,
        cancel,
      });

      const item =
        itemMessage instanceof MissionItemInt
          ? itemMessage
          : MissionItems.fromLegacyItem(itemMessage);

      items.push(item);
      onProgress?.(items.length, countMessage.count, item);
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

  async clear({ missionType = MavMissionType.MAV_MISSION_TYPE_MISSION, cancel = null } = {}) {
    await this.session.send(
      new MissionClearAll(this.targetSystem, this.targetComponent, missionType),
    );

    const ack = await this.session.waitForMessageType(MissionAck, {
      fromSystemId: this.targetSystem,
      timeoutMs: this.operationTimeoutMs,
      cancel,
    });

    return ack.type;
  }

  async setCurrent(seq, { cancel = null } = {}) {
    cancel?.throwIfCancelled();
    await this.session.send(
      new MissionSetCurrent(seq, this.targetSystem, this.targetComponent),
    );
  }

  async setCurrentWithCommand(
    seq,
    { command = null, alsoSendCommand = true, resetMission = false, cancel = null } = {},
  ) {
    cancel?.throwIfCancelled();
    await this.setCurrent(seq, { cancel });

    let ack = null;
    if (alsoSendCommand && command != null) {
      ack = await command.setMissionCurrent(seq, { resetMission, cancel });
    }

    return new MissionSetCurrentResult({ sequence: seq, commandAck: ack });
  }

  _isItemRequest(message, seq, missionType) {
    if (message instanceof MissionRequestInt) {
      return message.seq === seq && message.mission_type === missionType;
    }
    if (message instanceof MissionRequest) {
      return message.seq === seq && message.mission_type === missionType;
    }
    return false;
  }
}

/** Vehicle-side mission protocol handler for embedding in autopilot code. */
export class MissionServer {
  constructor({ session, initialMission = null, missionType = MavMissionType.MAV_MISSION_TYPE_MISSION }) {
    this.session = session;
    this.missionType = missionType;
    this._items = [...(initialMission ?? [])];
    this._incoming = new Map();
    this._incomingCount = null;
    this._frameUnsub = this.session.frames.subscribe((frame) => void this._onFrame(frame));
  }

  get items() {
    return Object.freeze([...this._items]);
  }

  async close() {
    this._frameUnsub?.();
    this._frameUnsub = null;
  }

  replaceMission(items) {
    this._items = MissionItems.withSequentialSeq(items);
    this._incoming.clear();
    this._incomingCount = null;
  }

  async _onFrame(frame) {
    const message = frame.message;

    if (message instanceof MissionCount && this._targetsUs(message)) {
      if (message.mission_type !== this.missionType) {
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

    if (message instanceof MissionItemInt && this._targetsUs(message)) {
      if (message.mission_type !== this.missionType) {
        return;
      }
      await this._storeIncomingItem(frame, message);
      return;
    }

    if (message instanceof MissionItem && this._targetsUs(message)) {
      if (message.mission_type !== this.missionType) {
        return;
      }
      await this._storeIncomingItem(frame, MissionItems.fromLegacyItem(message));
      return;
    }

    if (message instanceof MissionRequestInt && this._targetsUs(message)) {
      await this._sendRequestedItem(frame, message.seq);
      return;
    }

    if (message instanceof MissionRequest && this._targetsUs(message)) {
      await this._sendRequestedItem(frame, message.seq);
      return;
    }

    if (message instanceof MissionRequestList && this._targetsUs(message)) {
      if (message.mission_type !== this.missionType) {
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

    if (message instanceof MissionClearAll && this._targetsUs(message)) {
      if (message.mission_type !== this.missionType) {
        return;
      }
      this._items = [];
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

  async _storeIncomingItem(frame, item) {
    this._incoming.set(item.seq, item);
    const expected = this._incomingCount;
    if (expected == null) {
      return;
    }

    if (this._incoming.size < expected) {
      await this._requestUploadItem(frame, item.seq + 1);
      return;
    }

    this._items = Array.from({ length: expected }, (_, index) => this._incoming.get(index));
    this._incoming.clear();
    this._incomingCount = null;
    await this._sendUploadAck(frame);
  }

  async _requestUploadItem(requestFrame, seq) {
    await this.session.send(
      new MissionRequestInt(
        seq,
        requestFrame.systemId,
        requestFrame.componentId,
        this.missionType,
      ),
    );
  }

  async _sendUploadAck(requestFrame) {
    await this.session.send(
      new MissionAck(
        requestFrame.systemId,
        requestFrame.componentId,
        MavMissionResult.MAV_MISSION_ACCEPTED,
        this.missionType,
      ),
    );
  }

  async _sendRequestedItem(requestFrame, seq) {
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

    await this.session.send(this._items[seq]);
  }

  _targetsUs(message) {
    const targetSystem = message.target_system;
    const targetComponent = message.target_component;
    return this._matchesTarget(targetSystem, targetComponent);
  }

  _matchesTarget(targetSystem, targetComponent) {
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
