import { SerialPort } from 'serialport';
import type { MavlinkLink } from '../../../generated/ts/protocols/mavlink_link.ts';

type ReceiveWaiter = {
  resolve: (value: IteratorResult<Uint8Array>) => void;
  reject: (error: unknown) => void;
};

/** [MavlinkLink] implementation over a cross-platform serial port. */
export class SerialMavlinkLink implements MavlinkLink {
  private readonly port: SerialPort;
  private readonly queue: Uint8Array[] = [];
  private readonly waiters: ReceiveWaiter[] = [];
  private closed = false;

  private constructor(port: SerialPort) {
    this.port = port;

    port.on('data', (data: Buffer) => {
      const chunk = new Uint8Array(data);
      const waiter = this.waiters.shift();
      if (waiter !== undefined) {
        waiter.resolve({ done: false, value: chunk });
        return;
      }
      this.queue.push(chunk);
    });

    port.on('error', (error) => {
      while (this.waiters.length > 0) {
        this.waiters.shift()!.reject(error);
      }
    });
  }

  /** Open [path] at [baudRate] (MAVLink SITL commonly uses 57600 or 115200). */
  static async open(
    path: string,
    baudRate = 57600,
  ): Promise<SerialMavlinkLink> {
    const port = new SerialPort({
      path,
      baudRate,
      dataBits: 8,
      parity: 'none',
      stopBits: 1,
      autoOpen: false,
    });

    await new Promise<void>((resolve, reject) => {
      port.open((error) => {
        if (error !== null && error !== undefined) {
          reject(error);
          return;
        }
        resolve();
      });
    });

    await new Promise<void>((resolve, reject) => {
      port.set({ dtr: true, rts: true }, (error) => {
        if (error !== null && error !== undefined) {
          reject(error);
          return;
        }
        resolve();
      });
    });

    return new SerialMavlinkLink(port);
  }

  readonly receive: AsyncIterable<Uint8Array> = {
    [Symbol.asyncIterator]: () => ({
      next: (): Promise<IteratorResult<Uint8Array>> => {
        if (this.closed && this.queue.length === 0) {
          return Promise.resolve({ done: true, value: undefined });
        }

        const queued = this.queue.shift();
        if (queued !== undefined) {
          return Promise.resolve({ done: false, value: queued });
        }

        return new Promise<IteratorResult<Uint8Array>>((resolve, reject) => {
          this.waiters.push({ resolve, reject });
        });
      },
    }),
  };

  async send(data: Uint8Array): Promise<void> {
    if (this.closed) {
      throw new Error('SerialMavlinkLink is closed');
    }

    await new Promise<void>((resolve, reject) => {
      this.port.write(Buffer.from(data), (error) => {
        if (error !== null && error !== undefined) {
          reject(error);
          return;
        }
        resolve();
      });
    });
  }

  async close(): Promise<void> {
    if (this.closed) {
      return;
    }
    this.closed = true;

    await new Promise<void>((resolve) => {
      this.port.close(() => resolve());
    });

    while (this.waiters.length > 0) {
      this.waiters.shift()!.resolve({ done: true, value: undefined });
    }
  }
}
