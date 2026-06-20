#include "mavlink_vehicle_client.h"

#include <stdlib.h>

#include "../mavlink.h"

mavlink_vehicle_client_t *mavlink_vehicle_client_create(
  mavlink_session_t *session,
  mavlink_node_t vehicle,
  int parameter_request_timeout_ms,
  int parameter_idle_timeout_ms,
  int mission_item_timeout_ms,
  int mission_operation_timeout_ms,
  int command_timeout_ms
) {
  mavlink_vehicle_client_t *client = (mavlink_vehicle_client_t *)calloc(1, sizeof(*client));
  if (client == NULL) {
    return NULL;
  }
  client->session = session;
  client->vehicle = vehicle;
  client->parameters = parameter_protocol_create(
    session,
    vehicle.system_id,
    vehicle.component_id,
    parameter_idle_timeout_ms,
    parameter_request_timeout_ms
  );
  client->mission = mission_protocol_create(
    session,
    vehicle.system_id,
    vehicle.component_id,
    mission_item_timeout_ms,
    mission_operation_timeout_ms
  );
  client->command = command_protocol_create(
    session,
    vehicle.system_id,
    vehicle.component_id,
    command_timeout_ms
  );
  return client;
}

void mavlink_vehicle_client_destroy(mavlink_vehicle_client_t *client) {
  if (client == NULL) {
    return;
  }
  parameter_protocol_destroy(client->parameters);
  mission_protocol_destroy(client->mission);
  command_protocol_destroy(client->command);
  free(client);
}

mavlink_gcs_t *mavlink_gcs_connect(
  const mavlink_dialect_t *dialect,
  mavlink_link_t *link,
  uint8_t system_id,
  uint8_t component_id,
  int heartbeat_interval_ms,
  int heartbeat_timeout_ms,
  int mavlink_version
) {
  mavlink_gcs_t *gcs = (mavlink_gcs_t *)calloc(1, sizeof(*gcs));
  if (gcs == NULL) {
    return NULL;
  }

  gcs->session = mavlink_session_create(
    dialect,
    link,
    system_id,
    component_id,
    (mavlink_version_t)mavlink_version
  );
  if (gcs->session == NULL) {
    free(gcs);
    return NULL;
  }

  heartbeat_t hb = {
    .custom_mode = 0,
    .type = MAV_TYPE_GCS,
    .autopilot = MAV_AUTOPILOT_INVALID,
    .base_mode = 0,
    .system_status = MAV_STATE_ACTIVE,
    .mavlink_version = (uint8_t)dialect->version,
  };
  uint8_t hb_payload[heartbeat_ENCODED_LENGTH];
  heartbeat_serialize(&hb, hb_payload);

  gcs->heartbeat_publisher = heartbeat_publisher_create(
    gcs->session,
    hb_payload,
    heartbeat_MSG_ID,
    heartbeat_CRC_EXTRA,
    heartbeat_ENCODED_LENGTH,
    heartbeat_interval_ms
  );
  gcs->heartbeat_monitor = heartbeat_monitor_create(gcs->session, heartbeat_timeout_ms);
  return gcs;
}

void mavlink_gcs_start(mavlink_gcs_t *gcs) {
  if (gcs == NULL) {
    return;
  }
  heartbeat_monitor_start(gcs->heartbeat_monitor);
  heartbeat_publisher_start(gcs->heartbeat_publisher);
}

void mavlink_gcs_stop_heartbeats(mavlink_gcs_t *gcs) {
  if (gcs == NULL) {
    return;
  }
  heartbeat_publisher_stop(gcs->heartbeat_publisher);
  heartbeat_monitor_stop(gcs->heartbeat_monitor);
}

mavlink_wait_result_t mavlink_gcs_wait_for_vehicle(
  mavlink_gcs_t *gcs,
  const uint8_t *exclude_system_ids,
  size_t exclude_count,
  int timeout_ms,
  mavlink_vehicle_client_t **out_client
) {
  if (gcs == NULL) {
    return MAVLINK_WAIT_ERROR;
  }
  mavlink_node_t vehicle;
  mavlink_wait_result_t result = heartbeat_monitor_wait_for_vehicle(
    gcs->heartbeat_monitor,
    exclude_system_ids,
    exclude_count,
    timeout_ms,
    NULL,
    &vehicle
  );
  if (result != MAVLINK_WAIT_OK) {
    return result;
  }
  if (out_client != NULL) {
    *out_client = mavlink_gcs_vehicle_client(gcs, vehicle);
  }
  return MAVLINK_WAIT_OK;
}

mavlink_vehicle_client_t *mavlink_gcs_vehicle_client(mavlink_gcs_t *gcs, mavlink_node_t vehicle) {
  if (gcs == NULL) {
    return NULL;
  }
  return mavlink_vehicle_client_create(gcs->session, vehicle, 10000, 2000, 10000, 30000, 10000);
}

void mavlink_gcs_close(mavlink_gcs_t *gcs) {
  if (gcs == NULL) {
    return;
  }
  mavlink_gcs_stop_heartbeats(gcs);
  if (gcs->session != NULL) {
    mavlink_session_close(gcs->session);
    gcs->session = NULL;
  }
}

void mavlink_gcs_destroy(mavlink_gcs_t *gcs) {
  if (gcs == NULL) {
    return;
  }
  mavlink_gcs_close(gcs);
  heartbeat_publisher_destroy(gcs->heartbeat_publisher);
  heartbeat_monitor_destroy(gcs->heartbeat_monitor);
  free(gcs);
}
