import {
  Heartbeat,
  MavAutopilot,
  MavState,
  MavType,
} from '../mavlink.js';
import { MavlinkCancelledException } from './mavlink_cancellation.js';
import { MavlinkTimeoutException } from './mavlink_session.js';

/** MAVLink node identity (system + component). */
export class MavlinkNode {
  constructor(systemId, componentId) {
    this.systemId = systemId;
    this.componentId = componentId;
  }

  equals(other) {
    return (
      this.systemId === other.systemId &&
      this.componentId === other.componentId
    );
  }

  toString() {
    return `MavlinkNode(${this.systemId}:${this.componentId})`;
  }
}

class NodeBroadcast {
  constructor() {
    this._listeners = new Set();
  }

  subscribe(listener) {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(node) {
    for (const listener of this._listeners) {
      listener(node);
    }
  }
}

class HeartbeatBroadcast {
  constructor() {
    this._listeners = new Set();
  }

  subscribe(listener) {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(state) {
    for (const listener of this._listeners) {
      listener(state);
    }
  }
}

/** Tracks remote HEARTBEAT messages and reports connect / disconnect events. */
export class HeartbeatMonitor {
  constructor({ session, timeoutMs = 5000, watch = null, watchSystemId = null }) {
    this.session = session;
    this.timeoutMs = timeoutMs;
    this.watch = watch;
    this.watchSystemId = watchSystemId;

    this._states = new Map();
    this._online = new Map();
    this._heartbeatEvents = new HeartbeatBroadcast();
    this._connectedEvents = new NodeBroadcast();
    this._disconnectedEvents = new NodeBroadcast();
    this._frameUnsub = null;
    this._watchdogTimer = null;
    this._running = false;
  }

  get onHeartbeat() {
    return this._asyncIterable(this._heartbeatEvents);
  }

  get onConnected() {
    return this._asyncIterable(this._connectedEvents);
  }

  get onDisconnected() {
    return this._asyncIterable(this._disconnectedEvents);
  }

  /** Start monitoring. Safe to call only once; use [stop] before restarting. */
  start() {
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
  async stop() {
    if (!this._running) {
      return;
    }
    this._running = false;
    this._frameUnsub?.();
    this._frameUnsub = null;
    if (this._watchdogTimer != null) {
      clearInterval(this._watchdogTimer);
      this._watchdogTimer = null;
    }
  }

  stateFor(node) {
    return this._states.get(this._nodeKey(node)) ?? null;
  }

  stateForIds(systemId, componentId) {
    return this.stateFor(new MavlinkNode(systemId, componentId));
  }

  isOnline(node) {
    return this._online.get(this._nodeKey(node)) ?? false;
  }

  isOnlineIds(systemId, componentId) {
    return this.isOnline(new MavlinkNode(systemId, componentId));
  }

  *onlineNodes() {
    for (const [key, online] of this._online.entries()) {
      if (!online) {
        continue;
      }
      const [systemId, componentId] = key.split(':').map(Number);
      yield new MavlinkNode(systemId, componentId);
    }
  }

  /** Wait until the first online vehicle heartbeat is observed. */
  waitForVehicle({ excludeSystemIds = null, timeoutMs = 60000, cancel = null } = {}) {
    cancel?.throwIfCancelled();

    for (const node of this.onlineNodes()) {
      if (excludeSystemIds == null || !excludeSystemIds.has(node.systemId)) {
        return Promise.resolve(node);
      }
    }

    const resolvedTimeoutMs = timeoutMs ?? 60_000;
    return new Promise((resolve, reject) => {
      const unsubscribe = this._connectedEvents.subscribe((node) => {
        if (excludeSystemIds != null && excludeSystemIds.has(node.systemId)) {
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
            resolvedTimeoutMs,
          ),
        );
      }, resolvedTimeoutMs);

      let cancelStopped = false;
      if (cancel != null) {
        if (cancel.isCancelled) {
          cleanup();
          reject(new MavlinkCancelledException());
          return;
        }
        void (async () => {
          for await (const _ of cancel.onCancel) {
            if (cancelStopped) {
              return;
            }
            cleanup();
            reject(new MavlinkCancelledException());
            return;
          }
        })();
      }

      const cleanup = () => {
        cancelStopped = true;
        unsubscribe();
        clearTimeout(timer);
      };
    });
  }

  _onFrame(frame, heartbeat) {
    const node = new MavlinkNode(frame.systemId, frame.componentId);
    if (!this._shouldWatch(node)) {
      return;
    }

    const key = this._nodeKey(node);
    const wasOnline = this._online.get(key) ?? false;
    const receivedAt = new Date();
    const tracked = {
      node,
      heartbeat,
      receivedAt,
      online: true,
      get ageMs() {
        return Date.now() - receivedAt.getTime();
      },
    };

    this._states.set(key, tracked);
    this._online.set(key, true);
    this._heartbeatEvents.emit(tracked);

    if (!wasOnline) {
      this._connectedEvents.emit(node);
    }
  }

  _checkTimeouts() {
    const now = Date.now();
    for (const key of [...this._states.keys()]) {
      const state = this._states.get(key);
      if (state == null) {
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

  _shouldWatch(node) {
    if (this.watch != null) {
      for (const watched of this.watch) {
        if (watched.equals(node)) {
          return true;
        }
      }
      return false;
    }
    if (this.watchSystemId != null) {
      return node.systemId === this.watchSystemId;
    }
    return true;
  }

  _nodeKey(node) {
    return `${node.systemId}:${node.componentId}`;
  }

  _asyncIterable(broadcaster) {
    return {
      [Symbol.asyncIterator]() {
        const queue = [];
        const waiters = [];
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
          async next() {
            if (queue.length > 0) {
              return { done: false, value: queue.shift() };
            }
            if (done) {
              return { done: true, value: undefined };
            }
            return new Promise((resolve) => {
              waiters.push(resolve);
            });
          },
          async return() {
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
  constructor({ session, heartbeat, intervalMs = 1000 }) {
    this.session = session;
    this.intervalMs = intervalMs;
    this._heartbeat = heartbeat;
    this._timer = null;
    this._running = false;
  }

  get heartbeat() {
    return this._heartbeat;
  }

  updateHeartbeat(heartbeat) {
    this._heartbeat = heartbeat;
  }

  mutateHeartbeat(transform) {
    this._heartbeat = transform(this._heartbeat);
  }

  start() {
    if (this._running) {
      return;
    }
    this._running = true;
    void this.sendOnce();
    this._timer = setInterval(() => void this.sendOnce(), this.intervalMs);
  }

  stop() {
    this._running = false;
    if (this._timer != null) {
      clearInterval(this._timer);
      this._timer = null;
    }
  }

  async sendOnce() {
    await this.session.send(this._heartbeat);
  }
}

/** Convenience factories for common HEARTBEAT payloads. */
export class HeartbeatTemplates {
  static gcs(mavlinkVersionOrOptions) {
    const mavlinkVersion =
      typeof mavlinkVersionOrOptions === 'object'
        ? mavlinkVersionOrOptions.mavlinkVersion
        : mavlinkVersionOrOptions;
    return new Heartbeat(
      0,
      MavType.MAV_TYPE_GCS,
      MavAutopilot.MAV_AUTOPILOT_INVALID,
      0,
      MavState.MAV_STATE_ACTIVE,
      mavlinkVersion,
    );
  }

  static autopilot({
    mavlinkVersion,
    type = MavType.MAV_TYPE_QUADROTOR,
    autopilot = MavAutopilot.MAV_AUTOPILOT_PX4,
    systemStatus = MavState.MAV_STATE_ACTIVE,
    customMode = 0,
    baseMode = 0,
  } = {}) {
    return new Heartbeat(customMode, type, autopilot, baseMode, systemStatus, mavlinkVersion);
  }

  static onboardApi(mavlinkVersionOrOptions) {
    const mavlinkVersion =
      typeof mavlinkVersionOrOptions === 'object'
        ? mavlinkVersionOrOptions.mavlinkVersion
        : mavlinkVersionOrOptions;
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
