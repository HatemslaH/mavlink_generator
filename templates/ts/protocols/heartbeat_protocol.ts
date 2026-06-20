import {
  Heartbeat,
  MavAutopilot,
  MavState,
  MavType,
} from '../mavlink';
import type { MavlinkFrame } from '../mavlink_frame';
import {
  MavlinkCancellationToken,
  MavlinkCancelledException,
} from './mavlink_cancellation';
import { MavlinkSession } from './mavlink_session';
import { MavlinkTimeoutException } from './mavlink_session';

/** MAVLink node identity (system + component). */
export class MavlinkNode {
  constructor(
    readonly systemId: number,
    readonly componentId: number,
  ) {}

  equals(other: MavlinkNode): boolean {
    return (
      this.systemId === other.systemId &&
      this.componentId === other.componentId
    );
  }

  toString(): string {
    return `MavlinkNode(${this.systemId}:${this.componentId})`;
  }
}

/** Last known heartbeat state for a remote node. */
export interface TrackedHeartbeat {
  readonly node: MavlinkNode;
  readonly heartbeat: Heartbeat;
  readonly receivedAt: Date;
  readonly online: boolean;
  readonly ageMs: number;
}

type NodeListener = (node: MavlinkNode) => void;
type HeartbeatListener = (state: TrackedHeartbeat) => void;

class NodeBroadcast {
  private readonly _listeners = new Set<NodeListener>();

  subscribe(listener: NodeListener): () => void {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(node: MavlinkNode): void {
    for (const listener of this._listeners) {
      listener(node);
    }
  }
}

class HeartbeatBroadcast {
  private readonly _listeners = new Set<HeartbeatListener>();

  subscribe(listener: HeartbeatListener): () => void {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(state: TrackedHeartbeat): void {
    for (const listener of this._listeners) {
      listener(state);
    }
  }
}

/** Tracks remote HEARTBEAT messages and reports connect / disconnect events. */
export class HeartbeatMonitor {
  readonly session: MavlinkSession;
  readonly timeoutMs: number;
  readonly watch?: ReadonlySet<MavlinkNode>;
  readonly watchSystemId?: number;

  private readonly _states = new Map<string, TrackedHeartbeat>();
  private readonly _online = new Map<string, boolean>();
  private readonly _heartbeatEvents = new HeartbeatBroadcast();
  private readonly _connectedEvents = new NodeBroadcast();
  private readonly _disconnectedEvents = new NodeBroadcast();
  private _frameUnsub: (() => void) | null = null;
  private _watchdogTimer: ReturnType<typeof setInterval> | null = null;
  private _running = false;

  constructor(options: {
    session: MavlinkSession;
    timeoutMs?: number;
    watch?: ReadonlySet<MavlinkNode>;
    watchSystemId?: number;
  }) {
    this.session = options.session;
    this.timeoutMs = options.timeoutMs ?? 5000;
    this.watch = options.watch;
    this.watchSystemId = options.watchSystemId;
  }

  /** Emitted on every received (or recovered) heartbeat update. */
  get onHeartbeat(): AsyncIterable<TrackedHeartbeat> {
    const events = this._heartbeatEvents;
    return this._asyncIterable(events);
  }

  /** Emitted when a watched node comes online (first heartbeat or recovery). */
  get onConnected(): AsyncIterable<MavlinkNode> {
    const events = this._connectedEvents;
    return this._asyncIterable(events);
  }

  /** Emitted when a watched node times out without heartbeats. */
  get onDisconnected(): AsyncIterable<MavlinkNode> {
    const events = this._disconnectedEvents;
    return this._asyncIterable(events);
  }

  /** Start monitoring. Safe to call only once; use [stop] before restarting. */
  start(): void {
    if (this._running) {
      return;
    }
    this._running = true;
    const subscription = this.session.listenMessage(
      (message, frame) => {
        this._onFrame(frame, message);
      },
      { messageType: Heartbeat },
    );
    this._frameUnsub = () => subscription.cancel();

    this._watchdogTimer = setInterval(
      () => this._checkTimeouts(),
      Math.max(1, Math.floor(this.timeoutMs / 3)),
    );
  }

  /** Stop monitoring and release timers/subscriptions. */
  async stop(): Promise<void> {
    if (!this._running) {
      return;
    }
    this._running = false;
    this._frameUnsub?.();
    this._frameUnsub = null;
    if (this._watchdogTimer !== null) {
      clearInterval(this._watchdogTimer);
      this._watchdogTimer = null;
    }
  }

  /** Returns the latest state for [node], or `null` if no heartbeat was seen. */
  stateFor(node: MavlinkNode): TrackedHeartbeat | null {
    return this._states.get(this._nodeKey(node)) ?? null;
  }

  /** Returns the latest state for [systemId]/[componentId]. */
  stateForIds(systemId: number, componentId: number): TrackedHeartbeat | null {
    return this.stateFor(new MavlinkNode(systemId, componentId));
  }

  /** Whether [node] is currently considered online. */
  isOnline(node: MavlinkNode): boolean {
    return this._online.get(this._nodeKey(node)) ?? false;
  }

  /** Whether [systemId]/[componentId] is currently considered online. */
  isOnlineIds(systemId: number, componentId: number): boolean {
    return this.isOnline(new MavlinkNode(systemId, componentId));
  }

  /** All nodes currently tracked as online. */
  *onlineNodes(): Generator<MavlinkNode> {
    for (const [key, online] of this._online.entries()) {
      if (!online) {
        continue;
      }
      const [systemId, componentId] = key.split(':').map(Number);
      yield new MavlinkNode(systemId!, componentId!);
    }
  }

  /** Wait until the first online vehicle heartbeat is observed. */
  async waitForVehicle(options: {
    excludeSystemIds?: ReadonlySet<number>;
    timeoutMs?: number;
    cancel?: MavlinkCancellationToken;
  } = {}): Promise<MavlinkNode> {
    options.cancel?.throwIfCancelled();

    for (const node of this.onlineNodes()) {
      if (
        options.excludeSystemIds === undefined ||
        !options.excludeSystemIds.has(node.systemId)
      ) {
        return node;
      }
    }

    const timeoutMs = options.timeoutMs ?? 60_000;
    return new Promise<MavlinkNode>((resolve, reject) => {
      const unsubscribe = this._connectedEvents.subscribe((node) => {
        if (
          options.excludeSystemIds !== undefined &&
          options.excludeSystemIds.has(node.systemId)
        ) {
          return;
        }
        cleanup();
        resolve(node);
      });

      const timer = setTimeout(() => {
        cleanup();
        reject(
          new MavlinkTimeoutException(
            'Timed out waiting for vehicle heartbeat',
            timeoutMs,
          ),
        );
      }, timeoutMs);

      let cancelUnsub: (() => void) | undefined;
      if (options.cancel !== undefined) {
        if (options.cancel.isCancelled) {
          cleanup();
          reject(new MavlinkCancelledException());
          return;
        }
        void (async () => {
          for await (const _ of options.cancel!.onCancel) {
            cleanup();
            reject(new MavlinkCancelledException());
            return;
          }
        })();
      }

      const cleanup = (): void => {
        unsubscribe();
        clearTimeout(timer);
        cancelUnsub?.();
      };
    });
  }

  private _onFrame(frame: MavlinkFrame, heartbeat: Heartbeat): void {
    const node = new MavlinkNode(frame.systemId, frame.componentId);
    if (!this._shouldWatch(node)) {
      return;
    }

    const key = this._nodeKey(node);
    const wasOnline = this._online.get(key) ?? false;
    const receivedAt = new Date();
    const tracked: TrackedHeartbeat = {
      node,
      heartbeat,
      receivedAt,
      online: true,
      ageMs: 0,
    };

    this._states.set(key, tracked);
    this._online.set(key, true);
    this._heartbeatEvents.emit(tracked);

    if (!wasOnline) {
      this._connectedEvents.emit(node);
    }
  }

  private _checkTimeouts(): void {
    const now = Date.now();
    for (const key of [...this._states.keys()]) {
      const state = this._states.get(key);
      if (state === undefined) {
        continue;
      }

      const ageMs = now - state.receivedAt.getTime();
      const timedOut = ageMs > this.timeoutMs;
      const wasOnline = this._online.get(key) ?? false;

      if (timedOut && wasOnline) {
        this._online.set(key, false);
        const node = state.node;
        this._disconnectedEvents.emit(node);
        this._heartbeatEvents.emit({
          node,
          heartbeat: state.heartbeat,
          receivedAt: state.receivedAt,
          online: false,
          ageMs,
        });
      }
    }
  }

  private _shouldWatch(node: MavlinkNode): boolean {
    if (this.watch !== undefined) {
      for (const watched of this.watch) {
        if (watched.equals(node)) {
          return true;
        }
      }
      return false;
    }
    if (this.watchSystemId !== undefined) {
      return node.systemId === this.watchSystemId;
    }
    return true;
  }

  private _nodeKey(node: MavlinkNode): string {
    return `${node.systemId}:${node.componentId}`;
  }

  private _asyncIterable<T>(
    broadcaster: { subscribe(listener: (value: T) => void): () => void },
  ): AsyncIterable<T> {
    return {
      [Symbol.asyncIterator]() {
        const queue: T[] = [];
        const waiters: Array<(result: IteratorResult<T>) => void> = [];
        let done = false;
        const unsubscribe = broadcaster.subscribe((value) => {
          const waiter = waiters.shift();
          if (waiter !== undefined) {
            waiter({ done: false, value });
            return;
          }
          queue.push(value);
        });

        return {
          async next(): Promise<IteratorResult<T>> {
            if (queue.length > 0) {
              return { done: false, value: queue.shift()! };
            }
            if (done) {
              return { done: true, value: undefined };
            }
            return new Promise<IteratorResult<T>>((resolve) => {
              waiters.push(resolve);
            });
          },
          async return(): Promise<IteratorResult<T>> {
            done = true;
            unsubscribe();
            return { done: true, value: undefined };
          },
        };
      },
    };
  }
}

/** Periodically sends HEARTBEAT on a [MavlinkSession]. */
export class HeartbeatPublisher {
  readonly session: MavlinkSession;
  readonly intervalMs: number;

  private _heartbeat: Heartbeat;
  private _timer: ReturnType<typeof setInterval> | null = null;
  private _running = false;

  constructor(options: {
    session: MavlinkSession;
    heartbeat: Heartbeat;
    intervalMs?: number;
  }) {
    this.session = options.session;
    this._heartbeat = options.heartbeat;
    this.intervalMs = options.intervalMs ?? 1000;
  }

  /** Payload sent on each heartbeat. Update fields via [updateHeartbeat]. */
  get heartbeat(): Heartbeat {
    return this._heartbeat;
  }

  /** Replace the heartbeat payload (e.g. change [MavState]). */
  updateHeartbeat(heartbeat: Heartbeat): void {
    this._heartbeat = heartbeat;
  }

  /** Apply [transform] to the current heartbeat payload. */
  mutateHeartbeat(transform: (current: Heartbeat) => Heartbeat): void {
    this._heartbeat = transform(this._heartbeat);
  }

  /** Start periodic transmission. */
  start(): void {
    if (this._running) {
      return;
    }
    this._running = true;
    void this.sendOnce();
    this._timer = setInterval(() => {
      void this.sendOnce();
    }, this.intervalMs);
  }

  /** Stop periodic transmission. */
  stop(): void {
    this._running = false;
    if (this._timer !== null) {
      clearInterval(this._timer);
      this._timer = null;
    }
  }

  /** Send one heartbeat immediately. */
  async sendOnce(): Promise<void> {
    await this.session.send(this._heartbeat);
  }
}

/** Convenience factories for common HEARTBEAT payloads. */
export class HeartbeatTemplates {
  private constructor() {}

  /** Ground control station heartbeat. */
  static gcs(mavlinkVersion: number): Heartbeat {
    return new Heartbeat(
      0,
      MavType.MAV_TYPE_GCS,
      MavAutopilot.MAV_AUTOPILOT_INVALID,
      0,
      MavState.MAV_STATE_ACTIVE,
      mavlinkVersion,
    );
  }

  /** Generic onboard autopilot heartbeat. */
  static autopilot(options: {
    mavlinkVersion: number;
    type?: MavType;
    autopilot?: MavAutopilot;
    systemStatus?: MavState;
    customMode?: number;
    baseMode?: number;
  }): Heartbeat {
    return new Heartbeat(
      options.customMode ?? 0,
      options.type ?? MavType.MAV_TYPE_QUADROTOR,
      options.autopilot ?? MavAutopilot.MAV_AUTOPILOT_PX4,
      options.baseMode ?? 0,
      options.systemStatus ?? MavState.MAV_STATE_ACTIVE,
      options.mavlinkVersion,
    );
  }

  /** Companion computer / onboard API heartbeat. */
  static onboardApi(mavlinkVersion: number): Heartbeat {
    return new Heartbeat(
      0,
      MavType.MAV_TYPE_ONBOARD_CONTROLLER,
      MavAutopilot.MAV_AUTOPILOT_INVALID,
      0,
      MavState.MAV_STATE_ACTIVE,
      mavlinkVersion,
    );
  }
}
