#ifndef MAVLINK_PROTOCOLS_MAVLINK_VEHICLE_CLIENT_H
#define MAVLINK_PROTOCOLS_MAVLINK_VEHICLE_CLIENT_H

#include "command_protocol.h"
#include "heartbeat_protocol.h"
#include "mission_protocol.h"
#include "parameter_protocol.h"

typedef struct mavlink_vehicle_client {
  mavlink_session_t *session;
  mavlink_node_t vehicle;
  parameter_protocol_t *parameters;
  mission_protocol_t *mission;
  command_protocol_t *command;
} mavlink_vehicle_client_t;

typedef struct mavlink_gcs {
  mavlink_session_t *session;
  heartbeat_publisher_t *heartbeat_publisher;
  heartbeat_monitor_t *heartbeat_monitor;
} mavlink_gcs_t;

mavlink_vehicle_client_t *mavlink_vehicle_client_create(
  mavlink_session_t *session,
  mavlink_node_t vehicle,
  int parameter_request_timeout_ms,
  int parameter_idle_timeout_ms,
  int mission_item_timeout_ms,
  int mission_operation_timeout_ms,
  int command_timeout_ms
);

void mavlink_vehicle_client_destroy(mavlink_vehicle_client_t *client);

mavlink_gcs_t *mavlink_gcs_connect(
  const mavlink_dialect_t *dialect,
  mavlink_link_t *link,
  uint8_t system_id,
  uint8_t component_id,
  int heartbeat_interval_ms,
  int heartbeat_timeout_ms,
  int mavlink_version
);

void mavlink_gcs_start(mavlink_gcs_t *gcs);
void mavlink_gcs_stop_heartbeats(mavlink_gcs_t *gcs);

mavlink_wait_result_t mavlink_gcs_wait_for_vehicle(
  mavlink_gcs_t *gcs,
  const uint8_t *exclude_system_ids,
  size_t exclude_count,
  int timeout_ms,
  mavlink_vehicle_client_t **out_client
);

mavlink_vehicle_client_t *mavlink_gcs_vehicle_client(mavlink_gcs_t *gcs, mavlink_node_t vehicle);
void mavlink_gcs_close(mavlink_gcs_t *gcs);
void mavlink_gcs_destroy(mavlink_gcs_t *gcs);

#endif
