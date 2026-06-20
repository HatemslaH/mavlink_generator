import { CommandProtocol } from './command_protocol.js';
import { HeartbeatMonitor, HeartbeatPublisher, HeartbeatTemplates } from './heartbeat_protocol.js';
import { MavlinkSession } from './mavlink_session.js';
import { MissionProtocol } from './mission_protocol.js';
import { ParameterProtocol } from './parameter_protocol.js';

/** Protocol clients bound to a single remote MAVLink vehicle. */
export class MavlinkVehicleClient {
  constructor({
    session,
    vehicle,
    parameterRequestTimeoutMs = 10000,
    parameterIdleTimeoutMs = 2000,
    missionItemTimeoutMs = 10000,
    missionOperationTimeoutMs = 30000,
    commandTimeoutMs = 10000,
  }) {
    this.session = session;
    this.vehicle = vehicle;
    this.parameters = new ParameterProtocol({
      session,
      targetSystem: vehicle.systemId,
      targetComponent: vehicle.componentId,
      requestTimeoutMs: parameterRequestTimeoutMs,
      idleTimeoutMs: parameterIdleTimeoutMs,
    });
    this.mission = new MissionProtocol({
      session,
      targetSystem: vehicle.systemId,
      targetComponent: vehicle.componentId,
      itemTimeoutMs: missionItemTimeoutMs,
      operationTimeoutMs: missionOperationTimeoutMs,
    });
    this.command = new CommandProtocol({
      session,
      targetSystem: vehicle.systemId,
      targetComponent: vehicle.componentId,
      defaultTimeoutMs: commandTimeoutMs,
    });
  }

  get targetSystem() {
    return this.vehicle.systemId;
  }

  get targetComponent() {
    return this.vehicle.componentId;
  }
}

/** Ground control station bootstrap: session, heartbeat publisher, and monitor. */
export class MavlinkGcs {
  constructor({ session, heartbeatPublisher, heartbeatMonitor }) {
    this.session = session;
    this.heartbeatPublisher = heartbeatPublisher;
    this.heartbeatMonitor = heartbeatMonitor;
  }

  start() {
    this.heartbeatMonitor.start();
    this.heartbeatPublisher.start();
  }

  async stopHeartbeats() {
    this.heartbeatPublisher.stop();
    await this.heartbeatMonitor.stop();
  }

  async waitForVehicle({ excludeSystemIds = null, timeoutMs = 60000 } = {}) {
    const node = await this.heartbeatMonitor.waitForVehicle({ excludeSystemIds, timeoutMs });
    return new MavlinkVehicleClient({ session: this.session, vehicle: node });
  }

  vehicleClient(vehicle) {
    return new MavlinkVehicleClient({ session: this.session, vehicle });
  }

  static connect({
    dialect,
    link,
    systemId = 255,
    componentId = 190,
    heartbeatIntervalMs = 1000,
    heartbeatTimeoutMs = 3000,
  }) {
    const session = new MavlinkSession({
      dialect,
      link,
      systemId,
      componentId,
    });

    return new MavlinkGcs({
      session,
      heartbeatPublisher: new HeartbeatPublisher({
        session,
        heartbeat: HeartbeatTemplates.gcs({ mavlinkVersion: dialect.version }),
        intervalMs: heartbeatIntervalMs,
      }),
      heartbeatMonitor: new HeartbeatMonitor({
        session,
        timeoutMs: heartbeatTimeoutMs,
      }),
    });
  }

  async close() {
    await this.stopHeartbeats();
    await this.session.close();
  }
}
