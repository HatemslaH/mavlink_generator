/** Simple pub/sub stream for protocol-layer events (no external deps). */

export class EventStream {
  constructor() {
    this._listeners = new Set();
  }

  subscribe(listener) {
    this._listeners.add(listener);
    return () => this._listeners.delete(listener);
  }

  emit(value) {
    for (const listener of [...this._listeners]) {
      listener(value);
    }
  }

  async *[Symbol.asyncIterator]() {
    const queue = [];
    let pending = null;

    const unsub = this.subscribe((value) => {
      if (pending) {
        const resolve = pending;
        pending = null;
        resolve(value);
      } else {
        queue.push(value);
      }
    });

    try {
      while (true) {
        if (queue.length > 0) {
          yield queue.shift();
        } else {
          yield await new Promise((resolve) => {
            pending = resolve;
          });
        }
      }
    } finally {
      unsub();
    }
  }
}

/**
 * Transport-agnostic MAVLink byte stream.
 *
 * Implement `send`, `receive`, and optionally `close` for any physical or
 * logical link (USB serial, UDP, TCP, WebSocket, in-memory loopback, etc.).
 *
 * @typedef {object} MavlinkLink
 * @property {(data: Uint8Array) => Promise<void>} send
 * @property {EventStream<Uint8Array>} receive
 * @property {() => Promise<void>} [close]
 */

/** In-memory link for tests and virtual examples. */
export class VirtualMavlinkBus {
  constructor() {
    this._endpoints = [];
  }

  /** Create a new endpoint on this bus. */
  createEndpoint() {
    const endpoint = new VirtualMavlinkEndpoint(this);
    this._endpoints.push(endpoint);
    return endpoint;
  }

  _deliver(data, sender) {
    for (const endpoint of this._endpoints) {
      if (endpoint !== sender) {
        endpoint._emit(data);
      }
    }
  }

  /** Close every endpoint on the bus. */
  async closeAll() {
    const endpoints = [...this._endpoints];
    for (const endpoint of endpoints) {
      await endpoint.close();
    }
  }
}

class VirtualMavlinkEndpoint {
  constructor(bus) {
    this._bus = bus;
    this._receive = new EventStream();
    this._closed = false;
  }

  get receive() {
    return this._receive;
  }

  async send(data) {
    if (this._closed) {
      throw new Error('VirtualMavlinkEndpoint is closed');
    }
    this._bus._deliver(new Uint8Array(data), this);
  }

  _emit(data) {
    if (!this._closed) {
      this._receive.emit(data);
    }
  }

  async close() {
    if (this._closed) {
      return;
    }
    this._closed = true;
    const index = this._bus._endpoints.indexOf(this);
    if (index >= 0) {
      this._bus._endpoints.splice(index, 1);
    }
  }
}
