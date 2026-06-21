import {
  MavComponent,
  MavParamType,
  ParamRequestList,
  ParamRequestRead,
  ParamSet,
  ParamValue,
} from '../mavlink';
import type { MavlinkFrame } from '../mavlink_frame';
import { MavlinkCancellationToken } from './mavlink_cancellation';
import { MavlinkSession, MavlinkTimeoutException } from './mavlink_session';
import { ParamCodec } from './param_codec';

/** Decoded onboard parameter entry. */
export interface ParamEntry {
  readonly id: string;
  readonly value: number;
  readonly type: MavParamType;
  readonly index: number;
  readonly count: number;
}

export type ParamProgressCallback = (
  entry: ParamEntry,
  received: number,
  expected: number,
) => void;

/** GCS-side MAVLink parameter protocol client. */
export class ParameterProtocol {
  readonly session: MavlinkSession;
  readonly targetSystem: number;
  readonly targetComponent: number;
  readonly idleTimeoutMs: number;
  readonly requestTimeoutMs: number;

  private readonly _cache = new Map<string, ParamEntry>();

  constructor(options: {
    session: MavlinkSession;
    targetSystem: number;
    targetComponent: number;
    idleTimeoutMs?: number;
    requestTimeoutMs?: number;
  }) {
    this.session = options.session;
    this.targetSystem = options.targetSystem;
    this.targetComponent = options.targetComponent;
    this.idleTimeoutMs = options.idleTimeoutMs ?? 500;
    this.requestTimeoutMs = options.requestTimeoutMs ?? 3000;
  }

  /** Last fetched or written parameters keyed by name (unmodifiable view). */
  get cache(): ReadonlyMap<string, ParamEntry> {
    return this._cache;
  }

  clearCache(): void {
    this._cache.clear();
  }

  typeForName(name: string): MavParamType | null {
    return this._cache.get(name)?.type ?? null;
  }

  static entryFromParamValue(message: ParamValue): ParamEntry {
    return {
      id: ParamCodec.paramIdToString(message.paramId),
      value: ParamCodec.decodeValue(message.paramValue, message.paramType),
      type: message.paramType,
      index: message.paramIndex,
      count: message.paramCount,
    };
  }

  /** Request and collect the full parameter set from the vehicle. */
  async fetchAll(options: {
    onProgress?: ParamProgressCallback;
    cancel?: MavlinkCancellationToken;
  } = {}): Promise<ParamEntry[]> {
    const entries: ParamEntry[] = [];
    for await (const entry of this.fetchAllStream({ cancel: options.cancel })) {
      entries.push(entry);
      options.onProgress?.call(
        undefined,
        entry,
        entries.length,
        entry.count,
      );
    }
    return entries;
  }

  /** Stream parameters as they arrive from the vehicle. */
  async *fetchAllStream(options: {
    cancel?: MavlinkCancellationToken;
  } = {}): AsyncIterable<ParamEntry> {
    options.cancel?.throwIfCancelled();

    await this.session.send(
      new ParamRequestList(this.targetSystem, this.targetComponent),
    );

    let expectedCount = -1;
    const seenIndices = new Set<number>();
    let timeoutRetries = 0;

    while (true) {
      options.cancel?.throwIfCancelled();

      let value;
      try {
        value = await this.session.waitForMessage({
          predicate: (message) => {
            if (!(message instanceof ParamValue)) {
              return false;
            }
            return !seenIndices.has(message.paramIndex);
          },
          fromSystemId: this.targetSystem,
          timeoutMs:
            expectedCount === -1 ? this.requestTimeoutMs : this.idleTimeoutMs,
          cancel: options.cancel,
        });
        timeoutRetries = 0;
      } catch (error) {
        if (error instanceof MavlinkTimeoutException) {
          timeoutRetries += 1;
          if (timeoutRetries > 5) {
            throw error;
          }

          if (expectedCount === -1) {
            await this.session.send(
              new ParamRequestList(this.targetSystem, this.targetComponent),
            );
          } else {
            for (let i = 0; i < expectedCount; i++) {
              if (!seenIndices.has(i)) {
                await this.session.send(
                  new ParamRequestRead(
                    i,
                    this.targetSystem,
                    this.targetComponent,
                    ParamCodec.paramIdFromString(''),
                  ),
                );
              }
            }
          }
          continue;
        }
        throw error;
      }

      const paramValue = value as ParamValue;
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
  }

  /** Read a single parameter by name (`paramIndex` = -1). */
  readByName(
    name: string,
    cancel?: MavlinkCancellationToken,
  ): Promise<ParamEntry> {
    return this.read({ paramId: name, cancel });
  }

  /** Read a single parameter by onboard index. */
  readByIndex(
    index: number,
    cancel?: MavlinkCancellationToken,
  ): Promise<ParamEntry> {
    return this.read({ paramIndex: index, cancel });
  }

  /** Read one parameter by id or index. */
  async read(options: {
    paramId?: string;
    paramIndex?: number;
    cancel?: MavlinkCancellationToken;
  }): Promise<ParamEntry> {
    const paramIndex = options.paramIndex ?? -1;
    if (options.paramId === undefined && paramIndex < 0) {
      throw new Error(
        'Either paramId or a non-negative paramIndex is required',
      );
    }

    await this.session.send(
      new ParamRequestRead(
        paramIndex,
        this.targetSystem,
        this.targetComponent,
        ParamCodec.paramIdFromString(options.paramId ?? ''),
      ),
    );

    const value = await this.session.waitForMessageType(ParamValue, {
      fromSystemId: this.targetSystem,
      timeoutMs: this.requestTimeoutMs,
      cancel: options.cancel,
    });

    const entry = ParameterProtocol.entryFromParamValue(value);
    this._remember(entry);
    return entry;
  }

  /** Write a parameter and wait for the broadcast [ParamValue] acknowledgment. */
  async write(options: {
    name: string;
    value: number;
    type: MavParamType;
    cancel?: MavlinkCancellationToken;
  }): Promise<ParamEntry> {
    await this.session.send(
      new ParamSet(
        ParamCodec.encodeValue(options.value, options.type),
        this.targetSystem,
        this.targetComponent,
        ParamCodec.paramIdFromString(options.name),
        options.type,
      ),
    );

    const ack = await this.session.waitForMessage({
      predicate: (message) => {
        if (!(message instanceof ParamValue)) {
          return false;
        }
        return ParamCodec.paramIdToString(message.paramId) === options.name;
      },
      fromSystemId: this.targetSystem,
      timeoutMs: this.requestTimeoutMs,
      cancel: options.cancel,
    });

    const entry = ParameterProtocol.entryFromParamValue(ack as ParamValue);
    this._remember(entry);
    return entry;
  }

  /** Write using [type] when provided, otherwise the cached type for [name]. */
  writeByName(
    name: string,
    value: number,
    options: { type?: MavParamType; cancel?: MavlinkCancellationToken } = {},
  ): Promise<ParamEntry> {
    const resolvedType =
      options.type ??
      this.typeForName(name) ??
      MavParamType.MAV_PARAM_TYPE_REAL32;
    return this.write({
      name,
      value,
      type: resolvedType,
      cancel: options.cancel,
    });
  }

  private _remember(entry: ParamEntry): void {
    this._cache.set(entry.id, entry);
  }
}

type ParamStoreValue = { value: number; type: MavParamType };

/** Vehicle-side parameter store handler for embedding in autopilot code. */
export class ParameterServer {
  readonly session: MavlinkSession;

  private readonly _values = new Map<string, ParamStoreValue>();
  private readonly _unsubscribe: () => void;

  constructor(options: {
    session: MavlinkSession;
    initialValues?: ReadonlyMap<string, ParamStoreValue>;
  }) {
    this.session = options.session;
    if (options.initialValues !== undefined) {
      for (const [name, entry] of options.initialValues.entries()) {
        this._values.set(name, { ...entry });
      }
    }
    const subscription = this.session.listenMessage((message, frame) => {
      void this._onFrame(frame, message);
    });
    this._unsubscribe = () => subscription.cancel();
  }

  get values(): ReadonlyMap<string, ParamStoreValue> {
    return this._values;
  }

  async close(): Promise<void> {
    this._unsubscribe();
  }

  set(name: string, value: number, type: MavParamType): void {
    this._values.set(name, { value, type });
  }

  private async _onFrame(
    frame: MavlinkFrame,
    message: import('../mavlink_message').MavlinkMessage,
  ): Promise<void> {
    if (!(message instanceof ParamRequestList) &&
      !(message instanceof ParamRequestRead) &&
      !(message instanceof ParamSet)) {
      return;
    }
    if (message instanceof ParamRequestList) {
      if (
        message.targetSystem !== this.session.systemId &&
        message.targetSystem !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      await this._broadcastAll();
      return;
    }

    if (message instanceof ParamRequestRead) {
      if (
        message.targetSystem !== this.session.systemId &&
        message.targetSystem !== MavComponent.MAV_COMP_ID_ALL
      ) {
        return;
      }
      const entry = this._resolveRead(message);
      if (entry !== null) {
        await this._sendValue(entry.name, entry.value, this._indexOf(entry.name));
      }
      return;
    }

    if (message instanceof ParamSet) {
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

  private async _broadcastAll(): Promise<void> {
    const names = [...this._values.keys()];
    for (let index = 0; index < names.length; index++) {
      const name = names[index]!;
      const entry = this._values.get(name);
      if (entry !== undefined) {
        await this._sendValue(name, entry, index);
      }
    }
  }

  private async _sendValue(
    name: string,
    entry: ParamStoreValue,
    index: number,
  ): Promise<void> {
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

  private _resolveRead(
    request: ParamRequestRead,
  ): { name: string; value: ParamStoreValue } | null {
    if (request.paramIndex >= 0) {
      const names = [...this._values.keys()];
      if (request.paramIndex >= names.length) {
        return null;
      }
      const name = names[request.paramIndex]!;
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

  private _indexOf(name: string): number {
    return [...this._values.keys()].indexOf(name);
  }
}
