import {
  Heartbeat,
  MavAutopilot,
  MavState,
  MavType,
} from '../mavlink.js';
import { MavlinkCancelledException } from './mavlink_cancellation.js';
import { MavlinkTimeoutException } from './mavlink_session.js';
import { EventStream } from './mavlink_link.js';

/** MAVLink node identity (system + component). */
export class MavlinkNode {
  constructor(systemId, componentId) {
    this.systemId = systemId;
    this.componentId = componentId;
  }

  equals(other) {
    return (
      other instanceof MavlinkNode &&
      other.systemId === this.systemId &&
      other.componentId === this.componentId
    );
  }

  toString() {
    return `MavlinkNode(${this.systemId}:${this.componentId})`;
  }
}

/** Last known heartbeat state for a remote node. */
export class TrackedHeartbeat {
  constructor({ node, heartbeat, receivedAt, online }) {
    this.node = node;
    this.heartbeat = heartbeat;
    this.receivedAt = receivedAt;
    this.online = online;
  }

  get ageMs() {
    return Date.now() - this.receivedAt.getTime();
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
    this.onHeartbeat = new EventStream();
    this.onConnected = new EventStream();
    this.onDisconnected = new EventStream();

    this._frameUnsub = null;
    this._watchdogTimer = null;
    this._running = false;
  }

  /** Start monitoring. Safe to call only once; use [stop] before restarting. */
  start() {
    if (this._running) {
      return;
    }
    this._running = true;
    this._frameUnsub = this.session.frames.subscribe((frame) => this._onFrame(frame));
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

  static _nodeKey(node) {
    return `${node.systemId}:${node.componentId}`;
  }

  stateFor(node) {
    return this._states.get(HeartbeatMonitor._nodeKey(node)) ?? null;
  }

  stateForIds(systemId, componentId) {
    return this.stateFor(new MavlinkNode(systemId, componentId));
  }

  isOnline(node) {
    return this._online.get(HeartbeatMonitor._nodeKey(node)) ?? false;
  }

  isOnlineIds(systemId, componentId) {
    return this.isOnline(new MavlinkNode(systemId, componentId));
  }

  get onlineNodes() {
    const nodes = [];
    for (const [key, online] of this._online.entries()) {
      if (online) {
        const [systemId, componentId] = key.split(':').map(Number);
        nodes.push(new MavlinkNode(systemId, componentId));
      }
    }
    return nodes;
  }

  /** Wait until the first online vehicle heartbeat is observed. */
  waitForVehicle({ excludeSystemIds = null, timeoutMs = 60000, cancel = null } = {}) {
    cancel?.throwIfCancelled();

    for (const node of this.onlineNodes) {
      if (excludeSystemIds == null || !excludeSystemIds.has(node.systemId)) {
        return Promise.resolve(node);
      }
    }

    return new Promise((resolve, reject) => {
      let settled = false;
      const finish = (fn, value) => {
        if (settled) {
          return;
        }
        settled = true;
        cleanup();
        fn(value);
      };

      const onConnectedUnsub = this.onConnected.subscribe((node) => {
        if (excludeSystemIds != null && excludeSystemIds.has(node.systemId)) {
          return;
        }
        finish(resolve, node);
      });

      const timer = setTimeout(() => {
        finish(reject, new MavlinkTimeoutException('Timed out waiting for vehicle heartbeat', timeoutMs));
      }, timeoutMs);

      let cancelUnsub = null;
      if (cancel != null) {
        if (cancel.isCancelled) {
          finish(reject, new MavlinkCancelledException());
          return;
        }
        cancelUnsub = cancel.onCancel.subscribe(() => {
          finish(reject, new MavlinkCancelledException());
        });
      }

      const cleanup = () => {
        clearTimeout(timer);
        onConnectedUnsub();
        cancelUnsub?.();
      };
    });
  }

  _onFrame(frame) {
    if (!(frame.message instanceof Heartbeat)) {
      return;
    }

    const node = new MavlinkNode(frame.systemId, frame.componentId);
    const nodeKey = HeartbeatMonitor._nodeKey(node);
    if (!this._shouldWatch(node)) {
      return;
    }

    const heartbeat = frame.message;
    const wasOnline = this._online.get(nodeKey) ?? false;
    const now = new Date();
    const tracked = new TrackedHeartbeat({
      node,
      heartbeat,
      receivedAt: now,
      online: true,
    });

    this._states.set(nodeKey, tracked);
    this._online.set(nodeKey, true);
    this.onHeartbeat.emit(tracked);

    if (!wasOnline) {
      this.onConnected.emit(node);
    }
  }

  _checkTimeouts() {
    const now = Date.now();
    for (const nodeKey of [...this._states.keys()]) {
      const state = this._states.get(nodeKey);
      if (state == null) {
        continue;
      }

      const timedOut = now - state.receivedAt.getTime() > this.timeoutMs;
      const wasOnline = this._online.get(nodeKey) ?? false;

      if (timedOut && wasOnline) {
        this._online.set(nodeKey, false);
        this.onDisconnected.emit(state.node);
        this.onHeartbeat.emit(
          new TrackedHeartbeat({
            node: state.node,
            heartbeat: state.heartbeat,
            receivedAt: state.receivedAt,
            online: false,
          }),
        );
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
  static gcs({ mavlinkVersion }) {
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

  static onboardApi({ mavlinkVersion }) {
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
