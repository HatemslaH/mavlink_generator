#ifndef MAVLINK_SITL_GCS_CONTEXT_H
#define MAVLINK_SITL_GCS_CONTEXT_H

#include "mavlink.h"
#include "protocols/protocols.h"

/// Ground control station identity (MAVLink convention).
#define GCS_SYSTEM_ID 255
#define GCS_COMPONENT_ID 190

typedef struct gcs_context {
  mavlink_gcs_t *gcs;
  mavlink_node_t vehicle;
  mavlink_vehicle_client_t *client;
  mavlink_cancellation_token_t *operation_cancel;
} gcs_context_t;

static inline mavlink_session_t *gcs_context_session(const gcs_context_t *ctx) {
  return ctx != NULL && ctx->gcs != NULL ? ctx->gcs->session : NULL;
}

static inline heartbeat_monitor_t *gcs_context_heartbeat_monitor(const gcs_context_t *ctx) {
  return ctx != NULL && ctx->gcs != NULL ? ctx->gcs->heartbeat_monitor : NULL;
}

static inline parameter_protocol_t *gcs_context_parameters(const gcs_context_t *ctx) {
  return ctx != NULL && ctx->client != NULL ? ctx->client->parameters : NULL;
}

static inline mission_protocol_t *gcs_context_mission(const gcs_context_t *ctx) {
  return ctx != NULL && ctx->client != NULL ? ctx->client->mission : NULL;
}

static inline command_protocol_t *gcs_context_command(const gcs_context_t *ctx) {
  return ctx != NULL && ctx->client != NULL ? ctx->client->command : NULL;
}

static inline uint8_t gcs_context_target_system(const gcs_context_t *ctx) {
  return ctx != NULL ? ctx->vehicle.system_id : 0;
}

static inline uint8_t gcs_context_target_component(const gcs_context_t *ctx) {
  return ctx != NULL ? ctx->vehicle.component_id : 0;
}

#endif
