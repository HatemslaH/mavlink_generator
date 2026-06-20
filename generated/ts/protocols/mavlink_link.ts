/** Transport-agnostic MAVLink byte stream. */

export interface MavlinkLink {
  /** Send raw MAVLink frame bytes to the remote peer. */
  send(data: Uint8Array): Promise<void>;

  /** Incoming raw bytes from the remote peer. */
  readonly receive: AsyncIterable<Uint8Array>;

  /** Release link resources. Default implementation is a no-op. */
  close(): Promise<void>;
}

type ReceiveWaiter = {
  resolve: (value: IteratorResult<Uint8Array>) => void;
  reject: (error: unknown) => void;
};

class VirtualMavlinkEndpoint implements MavlinkLink {
  private readonly _bus: VirtualMavlinkBus;
  private readonly _queue: Uint8Array[] = [];
  private readonly _waiters: ReceiveWaiter[] = [];
  private _closed = false;

  constructor(bus: VirtualMavlinkBus) {
    this._bus = bus;
  }

  readonly receive: AsyncIterable<Uint8Array> = {
    [Symbol.asyncIterator]: () => ({
      next: (): Promise<IteratorResult<Uint8Array>> => {
        if (this._closed && this._queue.length === 0) {
          return Promise.resolve({ done: true, value: undefined });
        }
        const queued = this._queue.shift();
        if (queued !== undefined) {
          return Promise.resolve({ done: false, value: queued });
        }
        return new Promise<IteratorResult<Uint8Array>>((resolve, reject) => {
          this._waiters.push({ resolve, reject });
        });
      },
    }),
  };

  async send(data: Uint8Array): Promise<void> {
    if (this._closed) {
      throw new Error('VirtualMavlinkEndpoint is closed');
    }
    this._bus._deliver(new Uint8Array(data), this);
  }

  async close(): Promise<void> {
    if (this._closed) {
      return;
    }
    this._closed = true;
    while (this._waiters.length > 0) {
      const waiter = this._waiters.shift()!;
      waiter.resolve({ done: true, value: undefined });
    }
    this._bus._removeEndpoint(this);
  }

  _emit(data: Uint8Array): void {
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
  private readonly _endpoints: VirtualMavlinkEndpoint[] = [];

  /** Create a new endpoint on this bus. */
  createEndpoint(): MavlinkLink {
    const endpoint = new VirtualMavlinkEndpoint(this);
    this._endpoints.push(endpoint);
    return endpoint;
  }

  _deliver(data: Uint8Array, sender: VirtualMavlinkEndpoint): void {
    for (const endpoint of this._endpoints) {
      if (endpoint !== sender) {
        endpoint._emit(data);
      }
    }
  }

  _removeEndpoint(endpoint: VirtualMavlinkEndpoint): void {
    const index = this._endpoints.indexOf(endpoint);
    if (index >= 0) {
      this._endpoints.splice(index, 1);
    }
  }

  /** Close every endpoint on the bus. */
  async closeAll(): Promise<void> {
    const endpoints = [...this._endpoints];
    for (const endpoint of endpoints) {
      await endpoint.close();
    }
  }
}
