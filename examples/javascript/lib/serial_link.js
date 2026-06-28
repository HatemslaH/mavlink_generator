import { SerialPort } from 'serialport';

/** [MavlinkLink] implementation over a serial/COM port (serialport). */
export class SerialMavlinkLink {
  constructor(port) {
    this.port = port;
    this.queue = [];
    this.waiters = [];
    this.closed = false;

    port.on('data', (data) => {
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
        this.waiters.shift().reject(error);
      }
    });

    this.receive = {
      [Symbol.asyncIterator]: () => ({
        next: () => {
          if (this.closed && this.queue.length === 0) {
            return Promise.resolve({ done: true, value: undefined });
          }

          const queued = this.queue.shift();
          if (queued !== undefined) {
            return Promise.resolve({ done: false, value: queued });
          }

          return new Promise((resolve, reject) => {
            this.waiters.push({ resolve, reject });
          });
        },
      }),
    };
  }

  /** Open [portName] at [baudRate] (MAVLink SITL commonly uses 57600 or 115200). */
  static async open(portName, baudRate = 57600) {
    const port = new SerialPort({
      path: portName,
      baudRate,
      dataBits: 8,
      parity: 'none',
      stopBits: 1,
      autoOpen: false,
    });

    await new Promise((resolve, reject) => {
      port.open((error) => {
        if (error != null) {
          reject(error);
          return;
        }
        resolve();
      });
    });

    await new Promise((resolve, reject) => {
      port.set({ dtr: true, rts: true }, (error) => {
        if (error != null) {
          reject(error);
          return;
        }
        resolve();
      });
    });

    return new SerialMavlinkLink(port);
  }

  async send(data) {
    if (this.closed) {
      throw new Error('SerialMavlinkLink is closed');
    }

    await new Promise((resolve, reject) => {
      this.port.write(Buffer.from(data), (error) => {
        if (error != null) {
          reject(error);
          return;
        }
        resolve();
      });
    });
  }

  async close() {
    if (this.closed) {
      return;
    }
    this.closed = true;

    await new Promise((resolve) => {
      this.port.close(() => resolve());
    });

    while (this.waiters.length > 0) {
      this.waiters.shift().resolve({ done: true, value: undefined });
    }
  }
}
