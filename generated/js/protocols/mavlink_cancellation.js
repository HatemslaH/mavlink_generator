import { EventStream } from './mavlink_link.js';

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
    this._onCancel = new EventStream();
  }

  get isCancelled() {
    return this._cancelled;
  }

  /** Fires once when [cancel] is called. */
  get onCancel() {
    return this._onCancel;
  }

  cancel() {
    if (this._cancelled) {
      return;
    }
    this._cancelled = true;
    this._onCancel.emit(null);
  }

  throwIfCancelled() {
    if (this._cancelled) {
      throw new MavlinkCancelledException();
    }
  }

  dispose() {
    // Listeners are removed when unsubscribed; nothing to release.
  }
}
