/** Thrown when a MAVLink wait or long-running protocol operation is cancelled. */
export class MavlinkCancelledException extends Error {
  constructor(message = 'Operation cancelled') {
    super(message);
    this.name = 'MavlinkCancelledException';
  }
}

/** Cooperative cancellation token for session waits and protocol flows. */
export class MavlinkCancellationToken {
  constructor() {
    this._cancelled = false;
    this._listeners = new Set();
  }

  get isCancelled() {
    return this._cancelled;
  }

  /** Fires once when [cancel] is called. */
  get onCancel() {
    const token = this;
    return {
      [Symbol.asyncIterator]() {
        if (token._cancelled) {
          return {
            async next() {
              return { done: true, value: undefined };
            },
          };
        }
        return {
          async next() {
            return new Promise((resolve) => {
              if (token._cancelled) {
                resolve({ done: true, value: undefined });
                return;
              }
              const listener = () => {
                token._listeners.delete(listener);
                resolve({ done: false, value: undefined });
              };
              token._listeners.add(listener);
            });
          },
        };
      },
    };
  }

  cancel() {
    if (this._cancelled) {
      return;
    }
    this._cancelled = true;
    for (const listener of [...this._listeners]) {
      listener();
    }
    this._listeners.clear();
  }

  throwIfCancelled() {
    if (this._cancelled) {
      throw new MavlinkCancelledException();
    }
  }

  dispose() {
    this._listeners.clear();
  }
}
