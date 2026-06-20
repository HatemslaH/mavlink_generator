import type { MavlinkDialect } from '../mavlink_dialect';
import { MavlinkFrame } from '../mavlink_frame';
import type { MavlinkMessage } from '../mavlink_message';
import { MavlinkParser } from '../mavlink_parser';
import { MavlinkVersion } from '../mavlink_version';
import {
  MavlinkCancellationToken,
  MavlinkCancelledException,
} from './mavlink_cancellation';
import type { MavlinkLink } from './mavlink_link';

/** Thrown when an expected MAVLink message is not received in time. */
export class MavlinkTimeoutException extends Error {
  constructor(
    message: string,
    readonly timeoutMs: number,
  ) {
    super(`${message} (timeout: ${timeoutMs}ms)`);
    this.name = 'MavlinkTimeoutException';
  }
}

/** Handle returned by [MavlinkSession.listenMessage]; call [cancel] to unsubscribe. */
export class MavlinkMessageSubscription {
  private readonly _cancel: () => void;
  private _active = true;

  constructor(cancel: () => void) {
    this._cancel = cancel;
  }

  get isActive(): boolean {
    return this._active;
  }

  cancel(): void {
    if (!this._active) {
      return;
    }
    this._active = false;
    this._cancel();
  }
}

export type MessageClass<T extends MavlinkMessage = MavlinkMessage> = {
  readonly MSG_ID: number;
  new (...args: never[]): T;
};

type FrameListener = (frame: MavlinkFrame) => void;

class FrameBroadcast {
  private readonly _listeners = new Set<FrameListener>();
  private readonly _buffer: MavlinkFrame[] = [];

  subscribe(listener: FrameListener): () => void {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  emit(frame: MavlinkFrame): void {
    this._buffer.push(frame);
    if (this._buffer.length > 64) {
      this._buffer.shift();
    }
    for (const listener of this._listeners) {
      listener(frame);
    }
  }

  snapshot(): readonly MavlinkFrame[] {
    return [...this._buffer];
  }
}

type PendingFrameWait = {
  predicate: (frame: MavlinkFrame) => boolean;
  resolve: (frame: MavlinkFrame) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
  cancel?: MavlinkCancellationToken;
  cancelUnsub?: () => void;
};

export interface MavlinkSessionOptions {
  dialect: MavlinkDialect;
  link: MavlinkLink;
  systemId: number;
  componentId: number;
  version?: MavlinkVersion;
}

export interface FrameFilterOptions {
  fromSystemId?: number;
  fromComponentId?: number;
}

export interface WaitOptions extends FrameFilterOptions {
  timeoutMs?: number;
  cancel?: MavlinkCancellationToken;
}

export interface ListenMessageOptions<T extends MavlinkMessage>
  extends FrameFilterOptions {
  /** Message class for runtime type filtering (TypeScript). */
  messageType?: MessageClass<T>;
}

/** Framing, sequencing, and message dispatch over a [MavlinkLink]. */
export class MavlinkSession {
  readonly dialect: MavlinkDialect;
  readonly systemId: number;
  readonly componentId: number;
  readonly version: MavlinkVersion;

  private readonly _link: MavlinkLink;
  private readonly _parser: MavlinkParser;
  private readonly _frames = new FrameBroadcast();
  private readonly _pendingWaits: PendingFrameWait[] = [];
  private _sequence = 0;
  private _closed = false;
  private _receiveTask: Promise<void> | null = null;

  constructor(options: MavlinkSessionOptions) {
    this.dialect = options.dialect;
    this._link = options.link;
    this.systemId = options.systemId;
    this.componentId = options.componentId;
    this.version = options.version ?? MavlinkVersion.V2;
    this._parser = new MavlinkParser(this.dialect);
    this._receiveTask = this._consumeReceive();
  }

  /** All frames parsed from the link (before filtering). */
  get frames(): AsyncIterable<MavlinkFrame> {
    const frames = this._frames;
    return {
      [Symbol.asyncIterator]() {
        const queue: MavlinkFrame[] = [];
        const waiters: Array<(result: IteratorResult<MavlinkFrame>) => void> = [];
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
          async next(): Promise<IteratorResult<MavlinkFrame>> {
            if (queue.length > 0) {
              return { done: false, value: queue.shift()! };
            }
            if (done) {
              return { done: true, value: undefined };
            }
            return new Promise<IteratorResult<MavlinkFrame>>((resolve) => {
              waiters.push(resolve);
            });
          },
          async return(): Promise<IteratorResult<MavlinkFrame>> {
            done = true;
            unsubscribe();
            return { done: true, value: undefined };
          },
        };
      },
    };
  }

  /** Typed message stream filtered by [fromSystemId] / [fromComponentId]. */
  onMessage<T extends MavlinkMessage>(
    options: ListenMessageOptions<T> = {},
  ): AsyncIterable<T> {
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
          yield frame.message as T;
        }
      },
    };
  }

  /** Message stream filtered by MAVLink message id. */
  subscribeMessageId(
    messageId: number,
    options: FrameFilterOptions = {},
  ): AsyncIterable<MavlinkMessage> {
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

  /** Register a callback for messages of type [T]. Returns a subscription handle. */
  listenMessage<T extends MavlinkMessage>(
    onData: (message: T, frame: MavlinkFrame) => void,
    options: ListenMessageOptions<T> = {},
  ): MavlinkMessageSubscription {
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
      onData(frame.message as T, frame);
    });
    return new MavlinkMessageSubscription(unsubscribe);
  }

  /** Send a typed MAVLink message as a framed packet. */
  async send(message: MavlinkMessage): Promise<void> {
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
  waitForFrame(options: {
    predicate: (frame: MavlinkFrame) => boolean;
    timeoutMs?: number;
    cancel?: MavlinkCancellationToken;
  }): Promise<MavlinkFrame> {
    const timeoutMs = options.timeoutMs ?? 5000;
    options.cancel?.throwIfCancelled();

    return new Promise<MavlinkFrame>((resolve, reject) => {
      const wait: PendingFrameWait = {
        predicate: options.predicate,
        resolve,
        reject,
        timer: setTimeout(() => {
          this._removePendingWait(wait);
          reject(
            new MavlinkTimeoutException(
              'Timed out waiting for frame',
              timeoutMs,
            ),
          );
        }, timeoutMs),
        cancel: options.cancel,
      };

      if (options.cancel !== undefined) {
        if (options.cancel.isCancelled) {
          clearTimeout(wait.timer);
          reject(new MavlinkCancelledException());
          return;
        }
        wait.cancelUnsub = this._watchCancel(options.cancel, wait);
      }

      for (const frame of this._frames.snapshot()) {
        if (!options.predicate(frame)) {
          continue;
        }
        clearTimeout(wait.timer);
        wait.cancelUnsub?.();
        resolve(frame);
        return;
      }

      this._pendingWaits.push(wait);
    });
  }

  /** Wait for the first message matching [predicate]. */
  async waitForMessage(options: {
    predicate: (message: MavlinkMessage) => boolean;
    fromSystemId?: number;
    fromComponentId?: number;
    timeoutMs?: number;
    cancel?: MavlinkCancellationToken;
  }): Promise<MavlinkMessage> {
    const frame = await this.waitForFrame({
      predicate: (frame) => {
        if (
          options.fromSystemId !== undefined &&
          frame.systemId !== options.fromSystemId
        ) {
          return false;
        }
        if (
          options.fromComponentId !== undefined &&
          frame.componentId !== options.fromComponentId
        ) {
          return false;
        }
        return options.predicate(frame.message);
      },
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    });
    return frame.message;
  }

  /** Wait for the first message of type [T]. */
  waitForMessageType<T extends MavlinkMessage>(
    messageType: MessageClass<T>,
    options: WaitOptions = {},
  ): Promise<T> {
    return this.waitForMessage({
      predicate: (message) => message.mavlinkMessageId === messageType.MSG_ID,
      fromSystemId: options.fromSystemId,
      fromComponentId: options.fromComponentId,
      timeoutMs: options.timeoutMs,
      cancel: options.cancel,
    }) as Promise<T>;
  }

  async close(): Promise<void> {
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
    await this._link.close();
  }

  private async _consumeReceive(): Promise<void> {
    try {
      for await (const chunk of this._link.receive) {
        if (this._closed) {
          break;
        }
        const before = this._parser.frames.length;
        this._parser.parse(chunk);
        for (let index = before; index < this._parser.frames.length; index++) {
          this._onFrame(this._parser.frames[index]!);
        }
      }
    } catch {
      // Link closed or receive iterator ended.
    }
  }

  private _onFrame(frame: MavlinkFrame): void {
    if (this._closed) {
      return;
    }

    this._frames.emit(frame);

    for (const wait of [...this._pendingWaits]) {
      if (!wait.predicate(frame)) {
        continue;
      }
      clearTimeout(wait.timer);
      wait.cancelUnsub?.();
      this._removePendingWait(wait);
      wait.resolve(frame);
      break;
    }
  }

  private _removePendingWait(wait: PendingFrameWait): void {
    const index = this._pendingWaits.indexOf(wait);
    if (index >= 0) {
      this._pendingWaits.splice(index, 1);
    }
  }

  private _watchCancel(
    cancel: MavlinkCancellationToken,
    wait: PendingFrameWait,
  ): () => void {
    let stopped = false;
    const stop = (): void => {
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
