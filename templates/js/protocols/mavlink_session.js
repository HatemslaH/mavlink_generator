import { MavlinkFrame } from '../mavlink_frame.js';
import { MavlinkParser } from '../mavlink_parser.js';
import { MavlinkVersion } from '../mavlink_version.js';
import {
  MavlinkCancelledException,
} from './mavlink_cancellation.js';

/** Thrown when an expected MAVLink message is not received in time. */
export class MavlinkTimeoutException extends Error {
  constructor(message, timeoutMs) {
    super(`${message} (timeout: ${timeoutMs}ms)`);
    this.name = 'MavlinkTimeoutException';
    this.timeoutMs = timeoutMs;
  }
}

/** Handle returned by [MavlinkSession.listenMessage]; call [cancel] to unsubscribe. */
export class MavlinkMessageSubscription {
  constructor(cancel) {
    this._cancel = cancel;
    this._active = true;
  }

  get isActive() {
    return this._active;
  }

  cancel() {
    if (!this._active) {
      return;
    }
    this._active = false;
    this._cancel();
  }
}

class FrameBroadcast {
  constructor() {
    this._listeners = new Set();
    this._buffer = [];
  }

  subscribe(listener) {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(frame) {
    this._buffer.push(frame);
    if (this._buffer.length > 64) {
      this._buffer.shift();
    }
    for (const listener of this._listeners) {
      listener(frame);
    }
  }

  snapshot() {
    return [...this._buffer];
  }
}

/** Framing, sequencing, and message dispatch over a [MavlinkLink]. */
export class MavlinkSession {
  static _recentFrameCapacity = 64;

  constructor({ dialect, link, systemId, componentId, version = MavlinkVersion.V2 }) {
    this.dialect = dialect;
    this._link = link;
    this.systemId = systemId;
    this.componentId = componentId;
    this.version = version;

    this._parser = new MavlinkParser(dialect);
    this._frames = new FrameBroadcast();
    this._recentFrames = [];
    this._pendingWaits = [];
    this._sequence = 0;
    this._closed = false;
    this._receiveTask = this._consumeReceive();
  }

  /** All frames parsed from the link (before filtering). */
  get frames() {
    const frames = this._frames;
    return {
      [Symbol.asyncIterator]() {
        const queue = [];
        const waiters = [];
        let done = false;

        const unsubscribe = frames.subscribe((frame) => {
          const waiter = waiters.shift();
          if (waiter !== undefined) {
            waiter({ done: false, value: frame });
            return;
          }
          queue.push(frame);
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

  /** Typed message stream filtered by fromSystemId / fromComponentId. */
  onMessage(options = {}) {
    const session = this;
    return {
      async *[Symbol.asyncIterator]() {
        for await (const frame of session.frames) {
          if (
            options.fromSystemId !== undefined &&
            frame.systemId !== options.fromSystemId
          ) {
            continue;
          }
          if (
            options.fromComponentId !== undefined &&
            frame.componentId !== options.fromComponentId
          ) {
            continue;
          }
          if (
            options.messageType !== undefined &&
            frame.message.mavlinkMessageId !== options.messageType.MSG_ID
          ) {
            continue;
          }
          yield frame.message;
        }
      },
    };
  }

  /** Message stream filtered by MAVLink message id. */
  subscribeMessageId(messageId, options = {}) {
    const session = this;
    return {
      async *[Symbol.asyncIterator]() {
        for await (const frame of session.frames) {
          if (frame.message.mavlinkMessageId !== messageId) {
            continue;
          }
          if (
            options.fromSystemId !== undefined &&
            frame.systemId !== options.fromSystemId
          ) {
            continue;
          }
          if (
            options.fromComponentId !== undefined &&
            frame.componentId !== options.fromComponentId
          ) {
            continue;
          }
          yield frame.message;
        }
      },
    };
  }

  /** Register a callback for messages of type [messageType]. */
  listenMessage(onData, options = {}) {
    const unsubscribe = this._frames.subscribe((frame) => {
      if (
        options.fromSystemId !== undefined &&
        frame.systemId !== options.fromSystemId
      ) {
        return;
      }
      if (
        options.fromComponentId !== undefined &&
        frame.componentId !== options.fromComponentId
      ) {
        return;
      }
      if (
        options.messageType !== undefined &&
        frame.message.mavlinkMessageId !== options.messageType.MSG_ID
      ) {
        return;
      }
      onData(frame.message, frame);
    });
    return new MavlinkMessageSubscription(unsubscribe);
  }

  /** Send a typed MAVLink message as a framed packet. */
  async send(message) {
    if (this._closed) {
      throw new Error('MavlinkSession is closed');
    }

    const frame =
      this.version === MavlinkVersion.V2
        ? MavlinkFrame.v2(
            this._sequence++ & 0xff,
            this.systemId,
            this.componentId,
            message,
          )
        : MavlinkFrame.v1(
            this._sequence++ & 0xff,
            this.systemId,
            this.componentId,
            message,
          );

    await this._link.send(frame.serialize());
  }

  /** Wait for the first frame matching [predicate]. */
  waitForFrame({ predicate, timeoutMs = 5000, cancel = null }) {
    cancel?.throwIfCancelled();

    return new Promise((resolve, reject) => {
      const wait = {
        predicate,
        resolve,
        reject,
        timer: setTimeout(() => {
          this._removePendingWait(wait);
          reject(
            new MavlinkTimeoutException('Timed out waiting for frame', timeoutMs),
          );
        }, timeoutMs),
        cancel,
      };

      if (cancel != null) {
        if (cancel.isCancelled) {
          clearTimeout(wait.timer);
          reject(new MavlinkCancelledException());
          return;
        }
        wait.cancelUnsub = this._watchCancel(cancel, wait);
      }

      for (const frame of [...this._recentFrames]) {
        if (!predicate(frame)) {
          continue;
        }
        this._consumeRecentFrame(frame);
        clearTimeout(wait.timer);
        wait.cancelUnsub?.();
        resolve(frame);
        return;
      }

      this._pendingWaits.push(wait);
    });
  }

  /** Wait for the first message matching [predicate]. */
  async waitForMessage({
    predicate,
    fromSystemId,
    fromComponentId,
    timeoutMs = 5000,
    cancel = null,
  }) {
    const frame = await this.waitForFrame({
      predicate: (f) => {
        if (fromSystemId !== undefined && f.systemId !== fromSystemId) {
          return false;
        }
        if (fromComponentId !== undefined && f.componentId !== fromComponentId) {
          return false;
        }
        return predicate(f.message);
      },
      timeoutMs,
      cancel,
    });
    return frame.message;
  }

  /** Wait for the first message of type [messageType]. */
  async waitForMessageType(messageType, {
    fromSystemId,
    fromComponentId,
    timeoutMs = 5000,
    cancel = null,
  } = {}) {
    return this.waitForMessage({
      predicate: (message) => message.mavlinkMessageId === messageType.MSG_ID,
      fromSystemId,
      fromComponentId,
      timeoutMs,
      cancel,
    });
  }

  async close() {
    if (this._closed) {
      return;
    }
    this._closed = true;

    for (const wait of [...this._pendingWaits]) {
      clearTimeout(wait.timer);
      wait.cancelUnsub?.();
      wait.reject(new Error('MavlinkSession is closed'));
    }
    this._pendingWaits.length = 0;

    await this._receiveTask;
    await this._link.close?.();
  }

  async _consumeReceive() {
    try {
      for await (const chunk of this._link.receive) {
        if (this._closed) {
          break;
        }
        const before = this._parser.frames.length;
        this._parser.parse(chunk);
        for (let index = before; index < this._parser.frames.length; index++) {
          this._onFrame(this._parser.frames[index]);
        }
      }
    } catch {
      // Link closed or receive iterator ended.
    }
  }

  _onFrame(frame) {
    if (this._closed) {
      return;
    }

    this._frames.emit(frame);
    this._recentFrames.push(frame);
    if (this._recentFrames.length > MavlinkSession._recentFrameCapacity) {
      this._recentFrames.shift();
    }

    for (const wait of [...this._pendingWaits]) {
      if (!wait.predicate(frame)) {
        continue;
      }
      clearTimeout(wait.timer);
      wait.cancelUnsub?.();
      this._removePendingWait(wait);
      this._consumeRecentFrame(frame);
      wait.resolve(frame);
      break;
    }
  }

  _consumeRecentFrame(frame) {
    const index = this._recentFrames.indexOf(frame);
    if (index >= 0) {
      this._recentFrames.splice(index, 1);
    }
  }

  _removePendingWait(wait) {
    const index = this._pendingWaits.indexOf(wait);
    if (index >= 0) {
      this._pendingWaits.splice(index, 1);
    }
  }

  _watchCancel(cancel, wait) {
    let stopped = false;
    const stop = () => {
      if (stopped) {
        return;
      }
      stopped = true;
    };

    void (async () => {
      for await (const _ of cancel.onCancel) {
        if (stopped) {
          return;
        }
        this._removePendingWait(wait);
        clearTimeout(wait.timer);
        wait.reject(new MavlinkCancelledException());
        stop();
        return;
      }
    })();

    return stop;
  }
}
