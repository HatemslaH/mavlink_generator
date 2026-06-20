import type { MavlinkDialect } from '../mavlink_dialect';
import { HeartbeatTemplates } from './heartbeat_protocol';
import type { MavlinkLink } from './mavlink_link';
import { MavlinkSession } from './mavlink_session';
import { MavlinkNode, HeartbeatMonitor, HeartbeatPublisher } from './heartbeat_protocol';
import { CommandProtocol } from './command_protocol';
import { MissionProtocol } from './mission_protocol';
import { ParameterProtocol } from './parameter_protocol';

/** Protocol clients bound to a single remote MAVLink vehicle. */
export class MavlinkVehicleClient {
  readonly session: MavlinkSession;
  readonly vehicle: MavlinkNode;
  readonly parameters: ParameterProtocol;
  readonly mission: MissionProtocol;
  readonly command: CommandProtocol;

  constructor(options: {
    session: MavlinkSession;
    vehicle: MavlinkNode;
    parameterRequestTimeoutMs?: number;
    parameterIdleTimeoutMs?: number;
    missionItemTimeoutMs?: number;
    missionOperationTimeoutMs?: number;
    commandTimeoutMs?: number;
  }) {
    this.session = options.session;
    this.vehicle = options.vehicle;
    this.parameters = new ParameterProtocol({
      session: options.session,
      targetSystem: options.vehicle.systemId,
      targetComponent: options.vehicle.componentId,
      requestTimeoutMs: options.parameterRequestTimeoutMs ?? 10_000,
      idleTimeoutMs: options.parameterIdleTimeoutMs ?? 2000,
    });
    this.mission = new MissionProtocol({
      session: options.session,
      targetSystem: options.vehicle.systemId,
      targetComponent: options.vehicle.componentId,
      itemTimeoutMs: options.missionItemTimeoutMs ?? 10_000,
      operationTimeoutMs: options.missionOperationTimeoutMs ?? 30_000,
    });
    this.command = new CommandProtocol({
      session: options.session,
      targetSystem: options.vehicle.systemId,
      targetComponent: options.vehicle.componentId,
      defaultTimeoutMs: options.commandTimeoutMs ?? 10_000,
    });
  }

  get targetSystem(): number {
    return this.vehicle.systemId;
  }

  get targetComponent(): number {
    return this.vehicle.componentId;
  }
}

/** Ground control station bootstrap: session, heartbeat publisher, and monitor. */
export class MavlinkGcs {
  readonly session: MavlinkSession;
  readonly heartbeatPublisher: HeartbeatPublisher;
  readonly heartbeatMonitor: HeartbeatMonitor;

  constructor(options: {
    session: MavlinkSession;
    heartbeatPublisher: HeartbeatPublisher;
    heartbeatMonitor: HeartbeatMonitor;
  }) {
    this.session = options.session;
    this.heartbeatPublisher = options.heartbeatPublisher;
    this.heartbeatMonitor = options.heartbeatMonitor;
  }

  /** Start heartbeat publish/monitor loops. */
  start(): void {
    this.heartbeatMonitor.start();
    this.heartbeatPublisher.start();
  }

  /** Stop heartbeat publish/monitor loops. */
  async stopHeartbeats(): Promise<void> {
    this.heartbeatPublisher.stop();
    await this.heartbeatMonitor.stop();
  }

  /** Wait for the first vehicle and return a [MavlinkVehicleClient] for it. */
  async waitForVehicle(options: {
    excludeSystemIds?: ReadonlySet<number>;
    timeoutMs?: number;
  } = {}): Promise<MavlinkVehicleClient> {
    const node = await this.heartbeatMonitor.waitForVehicle(options);
    return new MavlinkVehicleClient({ session: this.session, vehicle: node });
  }

  /** Build a [MavlinkVehicleClient] for a known [vehicle] node. */
  vehicleClient(vehicle: MavlinkNode): MavlinkVehicleClient {
    return new MavlinkVehicleClient({ session: this.session, vehicle });
  }

  /** Factory for a typical GCS setup over [link]. */
  static connect(options: {
    dialect: MavlinkDialect;
    link: MavlinkLink;
    systemId?: number;
    componentId?: number;
    heartbeatIntervalMs?: number;
    heartbeatTimeoutMs?: number;
  }): MavlinkGcs {
    const session = new MavlinkSession({
      dialect: options.dialect,
      link: options.link,
      systemId: options.systemId ?? 255,
      componentId: options.componentId ?? 190,
    });

    return new MavlinkGcs({
      session,
      heartbeatPublisher: new HeartbeatPublisher({
        session,
        heartbeat: HeartbeatTemplates.gcs(options.dialect.version),
        intervalMs: options.heartbeatIntervalMs ?? 1000,
      }),
      heartbeatMonitor: new HeartbeatMonitor({
        session,
        timeoutMs: options.heartbeatTimeoutMs ?? 3000,
      }),
    });
  }

  async close(): Promise<void> {
    await this.stopHeartbeats();
    await this.session.close();
  }
}
