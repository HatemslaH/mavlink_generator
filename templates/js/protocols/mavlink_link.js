/** Transport-agnostic MAVLink byte stream. */

/**
 * @typedef {object} MavlinkLink
 * @property {(data: Uint8Array) => Promise<void>} send
 * @property {AsyncIterable<Uint8Array>} receive
 * @property {() => Promise<void>} [close]
 */

class VirtualMavlinkEndpoint {
  constructor(bus) {
    this._bus = bus;
    this._queue = [];
    this._waiters = [];
    this._closed = false;

    this.receive = {
      [Symbol.asyncIterator]: () => ({
        next: () => {
          if (this._closed && this._queue.length === 0) {
            return Promise.resolve({ done: true, value: undefined });
          }
          const queued = this._queue.shift();
          if (queued !== undefined) {
            return Promise.resolve({ done: false, value: queued });
          }
          return new Promise((resolve, reject) => {
            this._waiters.push({ resolve, reject });
          });
        },
      }),
    };
  }

  async send(data) {
    if (this._closed) {
      throw new Error('VirtualMavlinkEndpoint is closed');
    }
    this._bus._deliver(new Uint8Array(data), this);
  }

  async close() {
    if (this._closed) {
      return;
    }
    this._closed = true;
    while (this._waiters.length > 0) {
      const waiter = this._waiters.shift();
      waiter.resolve({ done: true, value: undefined });
    }
    this._bus._removeEndpoint(this);
  }

  _emit(data) {
    if (this._closed) {
      return;
    }
    const waiter = this._waiters.shift();
    if (waiter !== undefined) {
      waiter.resolve({ done: false, value: data });
      return;
    }
    this._queue.push(data);
  }
}

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

  _removeEndpoint(endpoint) {
    const index = this._endpoints.indexOf(endpoint);
    if (index >= 0) {
      this._endpoints.splice(index, 1);
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
