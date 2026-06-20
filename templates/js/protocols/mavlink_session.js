import { MavlinkFrame } from '../mavlink_frame.js';
import { MavlinkParser } from '../mavlink_parser.js';
import { MavlinkVersion } from '../mavlink_version.js';
import { MavlinkCancelledException } from './mavlink_cancellation.js';
import { EventStream } from './mavlink_link.js';

/** Thrown when an expected MAVLink message is not received in time. */
export class MavlinkTimeoutException extends Error {
  constructor(message, timeoutMs) {
    super(`${message} (timeout: ${timeoutMs}ms)`);
    this.name = 'MavlinkTimeoutException';
    this.message = message;
    this.timeoutMs = timeoutMs;
  }
}

/** Handle returned by [MavlinkSession.listenMessage]; call [cancel] to unsubscribe. */
export class MavlinkMessageSubscription {
  constructor(cancelFn) {
    this._cancel = cancelFn;
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

/** Framing, sequencing, and message dispatch over a [MavlinkLink]. */
export class MavlinkSession {
  constructor({ dialect, link, systemId, componentId, version = MavlinkVersion.V2 }) {
    this._dialect = dialect;
    this._link = link;
    this.systemId = systemId;
    this.componentId = componentId;
    this.version = version;

    this._parser = new MavlinkParser(dialect);
    this._frames = new EventStream();
    this._pendingWaits = [];
    this._recentFrames = [];
    this._recentFrameCapacity = 64;
    this._sequence = 0;
    this._closed = false;
    this._parsedFrameCount = 0;

    this._unsubscribeReceive = link.receive.subscribe((data) => this._onReceive(data));
  }

  get dialect() {
    return this._dialect;
  }

  /** All frames parsed from the link (before filtering). */
  get frames() {
    return this._frames;
  }

  /** Typed message stream filtered by fromSystemId / fromComponentId. */
  onMessage(messageClass, { fromSystemId, fromComponentId } = {}) {
    const stream = new EventStream();
    this._frames.subscribe((frame) => {
      if (fromSystemId != null && frame.systemId !== fromSystemId) {
        return;
      }
      if (fromComponentId != null && frame.componentId !== fromComponentId) {
        return;
      }
      if (frame.message instanceof messageClass) {
        stream.emit(frame.message);
      }
    });
    return stream;
  }

  /** Message stream filtered by MAVLink message id. */
  subscribeMessageId(messageId, { fromSystemId, fromComponentId } = {}) {
    const stream = new EventStream();
    this._frames.subscribe((frame) => {
      if (frame.message.mavlinkMessageId !== messageId) {
        return;
      }
      if (fromSystemId != null && frame.systemId !== fromSystemId) {
        return;
      }
      if (fromComponentId != null && frame.componentId !== fromComponentId) {
        return;
      }
      stream.emit(frame.message);
    });
    return stream;
  }

  /** Register a callback for messages of type [messageClass]. */
  listenMessage(messageClass, onData, { fromSystemId, fromComponentId } = {}) {
    const unsub = this._frames.subscribe((frame) => {
      if (fromSystemId != null && frame.systemId !== fromSystemId) {
        return;
      }
      if (fromComponentId != null && frame.componentId !== fromComponentId) {
        return;
      }
      if (frame.message instanceof messageClass) {
        onData(frame.message, frame);
      }
    });
    return new MavlinkMessageSubscription(unsub);
  }

  /** Send a typed MAVLink message as a framed packet. */
  async send(message) {
    if (this._closed) {
      throw new Error('MavlinkSession is closed');
    }

    const frame =
      this.version === MavlinkVersion.V2
        ? MavlinkFrame.v2(this._sequence++ & 0xff, this.systemId, this.componentId, message)
        : MavlinkFrame.v1(this._sequence++ & 0xff, this.systemId, this.componentId, message);

    await this._link.send(frame.serialize());
  }

  /** Wait for the first frame matching [predicate]. */
  waitForFrame({
    predicate,
    timeoutMs = 5000,
    cancel = null,
  }) {
    cancel?.throwIfCancelled();

    return new Promise((resolve, reject) => {
      const wait = { predicate, resolve, reject, timer: null, cancelUnsub: null };

      wait.timer = setTimeout(() => {
        this._removePendingWait(wait);
        reject(new MavlinkTimeoutException('Timed out waiting for frame', timeoutMs));
      }, timeoutMs);

      if (cancel != null) {
        if (cancel.isCancelled) {
          clearTimeout(wait.timer);
          reject(new MavlinkCancelledException());
          return;
        }
        wait.cancelUnsub = cancel.onCancel.subscribe(() => {
          this._removePendingWait(wait);
          reject(new MavlinkCancelledException());
        });
      }

      for (const frame of [...this._recentFrames]) {
        if (!predicate(frame)) {
          continue;
        }
        this._recentFrames.splice(this._recentFrames.indexOf(frame), 1);
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
        if (fromSystemId != null && f.systemId !== fromSystemId) {
          return false;
        }
        if (fromComponentId != null && f.componentId !== fromComponentId) {
          return false;
        }
        return predicate(f.message);
      },
      timeoutMs,
      cancel,
    });
    return frame.message;
  }

  /** Wait for the first message of type [messageClass]. */
  async waitForMessageType(messageClass, { fromSystemId, fromComponentId, timeoutMs = 5000, cancel = null } = {}) {
    return this.waitForMessage({
      predicate: (message) => message instanceof messageClass,
      fromSystemId,
      fromComponentId,
      timeoutMs,
      cancel,
    });
  }

  _onReceive(data) {
    const before = this._parser.frames.length;
    this._parser.parse(data);
    const frames = this._parser.frames;
    for (let i = before; i < frames.length; i++) {
      this._onFrame(frames[i]);
    }
    this._parsedFrameCount = frames.length;
  }

  _onFrame(frame) {
    if (this._closed) {
      return;
    }

    this._frames.emit(frame);
    this._recentFrames.push(frame);
    if (this._recentFrames.length > this._recentFrameCapacity) {
      this._recentFrames.shift();
    }

    for (const wait of [...this._pendingWaits]) {
      if (!wait.predicate(frame)) {
        continue;
      }

      clearTimeout(wait.timer);
      wait.cancelUnsub?.();
      this._removePendingWait(wait);
      const index = this._recentFrames.indexOf(frame);
      if (index >= 0) {
        this._recentFrames.splice(index, 1);
      }
      wait.resolve(frame);
      break;
    }
  }

  _removePendingWait(wait) {
    const index = this._pendingWaits.indexOf(wait);
    if (index >= 0) {
      this._pendingWaits.splice(index, 1);
    }
    clearTimeout(wait.timer);
    wait.cancelUnsub?.();
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
    this._pendingWaits = [];

    this._unsubscribeReceive?.();
    await this._link.close?.();
  }
}
