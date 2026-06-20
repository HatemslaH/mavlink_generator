import { SerialPort } from 'serialport';
import { EventStream } from '../../../generated/js/mavlink_protocols.js';

/** [MavlinkLink] implementation over a serial/COM port (serialport). */
export class SerialMavlinkLink {
  constructor(port) {
    this._port = port;
    this._receive = new EventStream();
    this._closed = false;

    this._port.on('data', (chunk) => {
      if (!this._closed) {
        this._receive.emit(new Uint8Array(chunk));
      }
    });
  }

  /** Open [portName] at [baudRate] (MAVLink SITL commonly uses 57600 or 115200). */
  static open(portName, { baudRate = 57600 } = {}) {
    const port = new SerialPort({
      path: portName,
      baudRate,
      dataBits: 8,
      parity: 'none',
      stopBits: 1,
      autoOpen: true,
    });
    return new SerialMavlinkLink(port);
  }

  get receive() {
    return this._receive;
  }

  async send(data) {
    if (this._closed) {
      throw new Error('SerialMavlinkLink is closed');
    }

    await new Promise((resolve, reject) => {
      this._port.write(Buffer.from(data), (error) => {
        if (error) {
          reject(error);
          return;
        }
        this._port.drain((drainError) => {
          if (drainError) {
            reject(drainError);
          } else {
            resolve();
          }
        });
      });
    });
  }

  async close() {
    if (this._closed) {
      return;
    }
    this._closed = true;
    await new Promise((resolve) => {
      this._port.close(() => resolve());
    });
  }
}
