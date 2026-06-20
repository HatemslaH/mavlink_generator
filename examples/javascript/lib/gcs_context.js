/** Ground control station identity (MAVLink convention). */
export const gcsSystemId = 255;
export const gcsComponentId = 190;

/**
 * Shared MAVLink GCS state for the interactive SITL example.
 *
 * @param {object} options
 * @param {import('../../../generated/js/mavlink_protocols.js').MavlinkGcs} options.gcs
 * @param {import('../../../generated/js/mavlink_protocols.js').MavlinkNode} options.vehicle
 * @param {import('../../../generated/js/mavlink_protocols.js').MavlinkVehicleClient} options.client
 */
export function createGcsContext({ gcs, vehicle, client }) {
  return {
    gcs,
    vehicle,
    client,
    operationCancel: null,
    get session() {
      return gcs.session;
    },
    get heartbeatMonitor() {
      return gcs.heartbeatMonitor;
    },
    get heartbeatPublisher() {
      return gcs.heartbeatPublisher;
    },
    get parameters() {
      return client.parameters;
    },
    get mission() {
      return client.mission;
    },
    get command() {
      return client.command;
    },
    get targetSystem() {
      return vehicle.systemId;
    },
    get targetComponent() {
      return vehicle.componentId;
    },
  };
}
