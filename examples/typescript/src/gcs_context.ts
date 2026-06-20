import type { CommandProtocol } from '../../../generated/ts/protocols/command_protocol.ts';
import type {
  HeartbeatMonitor,
  HeartbeatPublisher,
  MavlinkNode,
} from '../../../generated/ts/protocols/heartbeat_protocol.ts';
import type { MavlinkCancellationToken } from '../../../generated/ts/protocols/mavlink_cancellation.ts';
import type { MavlinkSession } from '../../../generated/ts/protocols/mavlink_session.ts';
import type {
  MavlinkGcs,
  MavlinkVehicleClient,
} from '../../../generated/ts/protocols/mavlink_vehicle_client.ts';
import type { MissionProtocol } from '../../../generated/ts/protocols/mission_protocol.ts';
import type { ParameterProtocol } from '../../../generated/ts/protocols/parameter_protocol.ts';

/** Ground control station identity (MAVLink convention). */
export const gcsSystemId = 255;
export const gcsComponentId = 190;

/** Shared MAVLink GCS state for the interactive SITL example. */
export class GcsContext {
  readonly gcs: MavlinkGcs;
  readonly vehicle: MavlinkNode;
  readonly client: MavlinkVehicleClient;

  /** Cancels in-flight parameter/mission operations (type `cancel` in CLI). */
  operationCancel: MavlinkCancellationToken | null = null;

  constructor(options: {
    gcs: MavlinkGcs;
    vehicle: MavlinkNode;
    client: MavlinkVehicleClient;
  }) {
    this.gcs = options.gcs;
    this.vehicle = options.vehicle;
    this.client = options.client;
  }

  get session(): MavlinkSession {
    return this.gcs.session;
  }

  get heartbeatMonitor(): HeartbeatMonitor {
    return this.gcs.heartbeatMonitor;
  }

  get heartbeatPublisher(): HeartbeatPublisher {
    return this.gcs.heartbeatPublisher;
  }

  get parameters(): ParameterProtocol {
    return this.client.parameters;
  }

  get mission(): MissionProtocol {
    return this.client.mission;
  }

  get command(): CommandProtocol {
    return this.client.command;
  }

  get targetSystem(): number {
    return this.vehicle.systemId;
  }

  get targetComponent(): number {
    return this.vehicle.componentId;
  }
}
