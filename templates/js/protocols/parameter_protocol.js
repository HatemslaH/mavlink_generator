import {
  MavComponent,
  MavParamType,
  ParamRequestList,
  ParamRequestRead,
  ParamSet,
  ParamValue,
} from '../mavlink.js';
import { ParamCodec } from './param_codec.js';

/** Decoded onboard parameter entry. */
export class ParamEntry {
  constructor({ id, value, type, index, count }) {
    this.id = id;
    this.value = value;
    this.type = type;
    this.index = index;
    this.count = count;
  }

  static fromParamValue(message) {
    return new ParamEntry({
      id: ParamCodec.paramIdToString(message.param_id),
      value: ParamCodec.decodeValue(message.param_value, message.param_type),
      type: message.param_type,
      index: message.param_index,
      count: message.param_count,
    });
  }
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

  get cache() {
    return Object.freeze(Object.fromEntries(this._cache));
  }

  clearCache() {
    this._cache.clear();
  }

  _remember(entry) {
    this._cache.set(entry.id, entry);
  }

  typeForName(name) {
    return this._cache.get(name)?.type ?? null;
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

    await this.session.send(
      new ParamRequestList(this.targetSystem, this.targetComponent),
    );

    let expectedCount = -1;
    const seenIndices = new Set();

    while (true) {
      cancel?.throwIfCancelled();

      const value = await this.session.waitForMessage({
        predicate: (message) => {
          if (!(message instanceof ParamValue)) {
            return false;
          }
          return !seenIndices.has(message.param_index);
        },
        fromSystemId: this.targetSystem,
        timeoutMs: expectedCount === -1 ? this.requestTimeoutMs : this.idleTimeoutMs,
        cancel,
      });

      seenIndices.add(value.param_index);

      if (expectedCount === -1) {
        expectedCount = value.param_count;
      }

      const entry = ParamEntry.fromParamValue(value);
      this._remember(entry);
      yield entry;

      if (seenIndices.size >= expectedCount) {
        break;
      }
    }
  }

  readByName(name, options = {}) {
    return this.read({ paramId: name, ...options });
  }

  readByIndex(index, options = {}) {
    return this.read({ paramIndex: index, ...options });
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
      timeoutMs: this.requestTimeoutMs,
      cancel,
    });

    const entry = ParamEntry.fromParamValue(value);
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
        if (!(message instanceof ParamValue)) {
          return false;
        }
        return ParamCodec.paramIdToString(message.param_id) === name;
      },
      fromSystemId: this.targetSystem,
      timeoutMs: this.requestTimeoutMs,
      cancel,
    });

    const entry = ParamEntry.fromParamValue(ack);
    this._remember(entry);
    return entry;
  }

  writeByName(name, value, { type = null, cancel = null } = {}) {
    const resolvedType = type ?? this.typeForName(name) ?? MavParamType.MAV_PARAM_TYPE_REAL32;
    return this.write({ name, value, type: resolvedType, cancel });
  }
}

/** Vehicle-side parameter store handler for embedding in autopilot code. */
export class ParameterServer {
  constructor({ session, initialValues = null }) {
    this.session = session;
    this._values = new Map(Object.entries(initialValues ?? {}));
    this._frameUnsub = this.session.frames.subscribe((frame) => void this._onFrame(frame));
  }

  get values() {
    return Object.freeze(Object.fromEntries(this._values));
  }

  async close() {
    this._frameUnsub?.();
    this._frameUnsub = null;
  }

  set(name, value, type) {
    this._values.set(name, { value, type });
  }

  async _onFrame(frame) {
    const message = frame.message;

    if (message instanceof ParamRequestList) {
      if (
        message.target_system !== this.session.systemId &&
        message.target_system !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      await this._broadcastAll();
      return;
    }

    if (message instanceof ParamRequestRead) {
      if (
        message.target_system !== this.session.systemId &&
        message.target_system !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      const entry = this._resolveRead(message);
      if (entry != null) {
        await this._sendValue(entry.name, entry.value, this._indexOf(entry.name));
      }
      return;
    }

    if (message instanceof ParamSet) {
      if (message.target_system !== this.session.systemId) {
        return;
      }
      const name = ParamCodec.paramIdToString(message.param_id);
      this._values.set(name, {
        value: ParamCodec.decodeValue(message.param_value, message.param_type),
        type: message.param_type,
      });
      await this._sendValue(name, this._values.get(name), this._indexOf(name));
    }
  }

  async _broadcastAll() {
    const names = [...this._values.keys()];
    for (let index = 0; index < names.length; index++) {
      await this._sendValue(names[index], this._values.get(names[index]), index);
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
    if (request.param_index >= 0) {
      const names = [...this._values.keys()];
      if (request.param_index >= names.length) {
        return null;
      }
      const name = names[request.param_index];
      return { name, value: this._values.get(name) };
    }

    const name = ParamCodec.paramIdToString(request.param_id);
    const entry = this._values.get(name);
    if (entry == null) {
      return null;
    }
    return { name, value: entry };
  }

  _indexOf(name) {
    return [...this._values.keys()].indexOf(name);
  }
}
