import {
  MavComponent,
  MavParamType,
  ParamRequestList,
  ParamRequestRead,
  ParamSet,
  ParamValue,
} from '../mavlink.js';
import { MavlinkMessage } from '../mavlink_message.js';
import { MavlinkTimeoutException } from './mavlink_session.js';
import { ParamCodec } from './param_codec.js';

function isParamValue(message) {
  return MavlinkMessage.isMessageOf(message, ParamValue);
}

/** GCS-side MAVLink parameter protocol client. */
export class ParameterProtocol {
  constructor({
    session,
    targetSystem,
    targetComponent,
    idleTimeoutMs = 500,
    requestTimeoutMs = 3000,
  }) {
    this.session = session;
    this.targetSystem = targetSystem;
    this.targetComponent = targetComponent;
    this.idleTimeoutMs = idleTimeoutMs;
    this.requestTimeoutMs = requestTimeoutMs;
    this._cache = new Map();
  }

  /** Last fetched or written parameters keyed by name (unmodifiable view). */
  get cache() {
    return this._cache;
  }

  clearCache() {
    this._cache.clear();
  }

  typeForName(name) {
    return this._cache.get(name)?.type ?? null;
  }

  static entryFromParamValue(message) {
    return {
      id: ParamCodec.paramIdToString(message.paramId),
      value: ParamCodec.decodeValue(message.paramValue, message.paramType),
      type: message.paramType,
      index: message.paramIndex,
      count: message.paramCount,
    };
  }

  async fetchAll({ onProgress = null, cancel = null } = {}) {
    const entries = [];
    for await (const entry of this.fetchAllStream({ cancel })) {
      entries.push(entry);
      onProgress?.(entry, entries.length, entry.count);
    }
    return entries;
  }

  async *fetchAllStream({ cancel = null } = {}) {
    cancel?.throwIfCancelled();

    const inbox = [];
    const subscription = this.session.listenMessage(
      (message) => {
        inbox.push(message);
      },
      {
        fromSystemId: this.targetSystem,
        fromComponentId: this.targetComponent,
        messageType: ParamValue,
      },
    );

    try {
      await this.session.send(
        new ParamRequestList(this.targetSystem, this.targetComponent),
      );

      let expectedCount = -1;
      const seenIndices = new Set();
      const retryCounts = new Map();
      let isRetrying = false;

      while (true) {
        cancel?.throwIfCancelled();

        let paramValue = this._takeNextParam(inbox, seenIndices);
        if (paramValue === null) {
          const timeoutMs =
            expectedCount === -1 || isRetrying
              ? this.requestTimeoutMs
              : this.idleTimeoutMs;
          try {
            paramValue = await this._waitForNextParam(
              inbox,
              seenIndices,
              timeoutMs,
              cancel,
            );
            isRetrying = false;
          } catch (error) {
            if (!(error instanceof MavlinkTimeoutException)) {
              throw error;
            }

            if (expectedCount === -1) {
              throw error;
            }

            const missingIndex = this._findMissingIndex(seenIndices, expectedCount);
            if (missingIndex === null) {
              break;
            }

            const retries = retryCounts.get(missingIndex) ?? 0;
            if (retries >= 3) {
              throw error;
            }
            retryCounts.set(missingIndex, retries + 1);
            isRetrying = true;

            await this.session.send(
              new ParamRequestRead(
                missingIndex,
                this.targetSystem,
                this.targetComponent,
                ParamCodec.paramIdFromString(''),
              ),
            );
            continue;
          }
        } else {
          isRetrying = false;
        }

        seenIndices.add(paramValue.paramIndex);

        if (expectedCount === -1) {
          expectedCount = paramValue.paramCount;
        }

        const entry = ParameterProtocol.entryFromParamValue(paramValue);
        this._remember(entry);
        yield entry;

        if (seenIndices.size >= expectedCount) {
          break;
        }
      }
    } finally {
      subscription.cancel();
    }
  }

  readByName(name, cancel = null) {
    return this.read({ paramId: name, cancel });
  }

  readByIndex(index, cancel = null) {
    return this.read({ paramIndex: index, cancel });
  }

  async read({ paramId = null, paramIndex = -1, cancel = null } = {}) {
    if (paramId == null && paramIndex < 0) {
      throw new Error('Either paramId or a non-negative paramIndex is required');
    }

    await this.session.send(
      new ParamRequestRead(
        paramIndex,
        this.targetSystem,
        this.targetComponent,
        ParamCodec.paramIdFromString(paramId ?? ''),
      ),
    );

    const value = await this.session.waitForMessageType(ParamValue, {
      fromSystemId: this.targetSystem,
      fromComponentId: this.targetComponent,
      timeoutMs: this.requestTimeoutMs,
      cancel,
    });

    const entry = ParameterProtocol.entryFromParamValue(value);
    this._remember(entry);
    return entry;
  }

  async write({ name, value, type, cancel = null }) {
    await this.session.send(
      new ParamSet(
        ParamCodec.encodeValue(value, type),
        this.targetSystem,
        this.targetComponent,
        ParamCodec.paramIdFromString(name),
        type,
      ),
    );

    const ack = await this.session.waitForMessage({
      predicate: (message) => {
        if (!isParamValue(message)) {
          return false;
        }
        return ParamCodec.paramIdToString(message.paramId) === name;
      },
      fromSystemId: this.targetSystem,
      fromComponentId: this.targetComponent,
      timeoutMs: this.requestTimeoutMs,
      cancel,
    });

    const entry = ParameterProtocol.entryFromParamValue(ack);
    this._remember(entry);
    return entry;
  }

  writeByName(name, value, { type = null, cancel = null } = {}) {
    const resolvedType =
      type ?? this.typeForName(name) ?? MavParamType.MAV_PARAM_TYPE_REAL32;
    return this.write({ name, value, type: resolvedType, cancel });
  }

  _remember(entry) {
    this._cache.set(entry.id, entry);
  }

  _takeNextParam(inbox, seenIndices) {
    while (inbox.length > 0) {
      const next = inbox.shift();
      if (!seenIndices.has(next.paramIndex)) {
        return next;
      }
    }
    return null;
  }

  async _waitForNextParam(inbox, seenIndices, timeoutMs, cancel) {
    const buffered = this._takeNextParam(inbox, seenIndices);
    if (buffered !== null) {
      return buffered;
    }

    const message = await this.session.waitForMessage({
      predicate: (candidate) => {
        if (!isParamValue(candidate)) {
          return false;
        }
        return !seenIndices.has(candidate.paramIndex);
      },
      fromSystemId: this.targetSystem,
      fromComponentId: this.targetComponent,
      timeoutMs,
      cancel,
    });

    return message;
  }

  _findMissingIndex(seenIndices, expectedCount) {
    for (let index = 0; index < expectedCount; index++) {
      if (!seenIndices.has(index)) {
        return index;
      }
    }
    return null;
  }
}

/** Vehicle-side parameter store handler for embedding in autopilot code. */
export class ParameterServer {
  constructor({ session, initialValues = null }) {
    this.session = session;
    this._values = new Map();
    if (initialValues != null) {
      const entries =
        initialValues instanceof Map
          ? initialValues.entries()
          : Object.entries(initialValues);
      for (const [name, entry] of entries) {
        this._values.set(name, { ...entry });
      }
    }
    const subscription = this.session.listenMessage((message, frame) => {
      void this._onFrame(frame, message);
    });
    this._unsubscribe = () => subscription.cancel();
  }

  get values() {
    return this._values;
  }

  async close() {
    this._unsubscribe();
  }

  set(name, value, type) {
    this._values.set(name, { value, type });
  }

  async _onFrame(frame, message) {
    if (
      !MavlinkMessage.isMessageOf(message, ParamRequestList) &&
      !MavlinkMessage.isMessageOf(message, ParamRequestRead) &&
      !MavlinkMessage.isMessageOf(message, ParamSet)
    ) {
      return;
    }

    if (MavlinkMessage.isMessageOf(message, ParamRequestList)) {
      if (
        message.targetSystem !== this.session.systemId &&
        message.targetSystem !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      await this._broadcastAll();
      return;
    }

    if (MavlinkMessage.isMessageOf(message, ParamRequestRead)) {
      if (
        message.targetSystem !== this.session.systemId &&
        message.targetSystem !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      const entry = this._resolveRead(message);
      if (entry != null) {
        await this._sendValue(entry.name, entry.value, this._indexOf(entry.name));
      }
      return;
    }

    if (MavlinkMessage.isMessageOf(message, ParamSet)) {
      if (message.targetSystem !== this.session.systemId) {
        return;
      }
      const name = ParamCodec.paramIdToString(message.paramId);
      this._values.set(name, {
        value: ParamCodec.decodeValue(message.paramValue, message.paramType),
        type: message.paramType,
      });
      const stored = this._values.get(name);
      if (stored !== undefined) {
        await this._sendValue(name, stored, this._indexOf(name));
      }
    }
  }

  async _broadcastAll() {
    const names = [...this._values.keys()];
    for (let index = 0; index < names.length; index++) {
      const name = names[index];
      const entry = this._values.get(name);
      if (entry !== undefined) {
        await this._sendValue(name, entry, index);
      }
    }
  }

  async _sendValue(name, entry, index) {
    await this.session.send(
      new ParamValue(
        ParamCodec.encodeValue(entry.value, entry.type),
        this._values.size,
        index,
        ParamCodec.paramIdFromString(name),
        entry.type,
      ),
    );
  }

  _resolveRead(request) {
    if (request.paramIndex >= 0) {
      const names = [...this._values.keys()];
      if (request.paramIndex >= names.length) {
        return null;
      }
      const name = names[request.paramIndex];
      const value = this._values.get(name);
      if (value === undefined) {
        return null;
      }
      return { name, value };
    }

    const name = ParamCodec.paramIdToString(request.paramId);
    const value = this._values.get(name);
    if (value === undefined) {
      return null;
    }
    return { name, value };
  }

  _indexOf(name) {
    return [...this._values.keys()].indexOf(name);
  }
}
