#ifndef MAVLINK_PROTOCOLS_HEARTBEAT_PROTOCOL_H
#define MAVLINK_PROTOCOLS_HEARTBEAT_PROTOCOL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "mavlink_cancellation.h"
#include "mavlink_session.h"

typedef struct mavlink_node {
  uint8_t system_id;
  uint8_t component_id;
} mavlink_node_t;

typedef struct heartbeat_monitor heartbeat_monitor_t;
typedef struct heartbeat_publisher heartbeat_publisher_t;

typedef struct tracked_heartbeat {
  mavlink_node_t node;
  void *heartbeat;
  uint64_t received_at_ms;
  bool online;
} tracked_heartbeat_t;

heartbeat_monitor_t *heartbeat_monitor_create(
  mavlink_session_t *session,
  int timeout_ms
);

void heartbeat_monitor_start(heartbeat_monitor_t *monitor);
void heartbeat_monitor_stop(heartbeat_monitor_t *monitor);

const tracked_heartbeat_t *heartbeat_monitor_state_for(
  const heartbeat_monitor_t *monitor,
  mavlink_node_t node
);

bool heartbeat_monitor_is_online(const heartbeat_monitor_t *monitor, mavlink_node_t node);

bool heartbeat_monitor_is_online_ids(
  const heartbeat_monitor_t *monitor,
  uint8_t system_id,
  uint8_t component_id
);

/// Wait until the first online vehicle heartbeat is observed. Monitor must be started.
mavlink_wait_result_t heartbeat_monitor_wait_for_vehicle(
  heartbeat_monitor_t *monitor,
  const uint8_t *exclude_system_ids,
  size_t exclude_count,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_node_t *out_vehicle
);

void heartbeat_monitor_destroy(heartbeat_monitor_t *monitor);

heartbeat_publisher_t *heartbeat_publisher_create(
  mavlink_session_t *session,
  const void *heartbeat,
  uint32_t message_id,
  uint8_t crc_extra,
  size_t encoded_length,
  int interval_ms
);

void heartbeat_publisher_start(heartbeat_publisher_t *publisher);
void heartbeat_publisher_stop(heartbeat_publisher_t *publisher);
int heartbeat_publisher_send_once(heartbeat_publisher_t *publisher);
void heartbeat_publisher_update_heartbeat(heartbeat_publisher_t *publisher, const void *heartbeat);
void heartbeat_publisher_destroy(heartbeat_publisher_t *publisher);

#endif
