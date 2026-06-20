/** Thrown when a MAVLink wait or long-running protocol operation is cancelled. */
export class MavlinkCancelledException extends Error {
  constructor(message = 'Operation cancelled') {
    super(message);
    this.name = 'MavlinkCancelledException';
  }
}

/** Cooperative cancellation token for [MavlinkSession] waits and protocol flows. */
export class MavlinkCancellationToken {
  private _cancelled = false;
  private readonly _listeners = new Set<() => void>();

  get isCancelled(): boolean {
    return this._cancelled;
  }

  /** Fires once when [cancel] is called. */
  get onCancel(): AsyncIterable<void> {
    const token = this;
    return {
      [Symbol.asyncIterator]() {
        if (token._cancelled) {
          return {
            async next(): Promise<IteratorResult<void>> {
              return { done: true, value: undefined };
            },
          };
        }
        return {
          async next(): Promise<IteratorResult<void>> {
            return new Promise<IteratorResult<void>>((resolve) => {
              if (token._cancelled) {
                resolve({ done: true, value: undefined });
                return;
              }
              const listener = (): void => {
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

  cancel(): void {
    if (this._cancelled) {
      return;
    }
    this._cancelled = true;
    for (const listener of [...this._listeners]) {
      listener();
    }
    this._listeners.clear();
  }

  throwIfCancelled(): void {
    if (this._cancelled) {
      throw new MavlinkCancelledException();
    }
  }

  dispose(): void {
    this._listeners.clear();
  }
}
