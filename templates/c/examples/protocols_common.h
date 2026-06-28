#ifndef MAVLINK_EXAMPLES_PROTOCOLS_COMMON_H
#define MAVLINK_EXAMPLES_PROTOCOLS_COMMON_H

#include "../mavlink_protocols.h"

/// Ground control station identity (MAVLink convention).
#define GCS_SYSTEM_ID 255
#define GCS_COMPONENT_ID 190

/// Simulated autopilot identity.
#define DRONE_SYSTEM_ID 1
#define DRONE_COMPONENT_ID 1

typedef struct virtual_mavlink_link {
  virtual_mavlink_bus_t *bus;
  mavlink_session_t *gcs;
  mavlink_session_t *drone;
  const mavlink_dialect_t *dialect;
} virtual_mavlink_link_t;

/// Create a linked GCS/drone pair over an in-memory MAVLink bus.
virtual_mavlink_link_t virtual_mavlink_link_create(const mavlink_dialect_t *dialect);

/// Tear down sessions and close the virtual bus.
void virtual_mavlink_link_close(virtual_mavlink_link_t *link);

#endif
