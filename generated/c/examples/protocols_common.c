#include "protocols_common.h"

#include <stdlib.h>

virtual_mavlink_link_t virtual_mavlink_link_create(const mavlink_dialect_t *dialect) {
  virtual_mavlink_link_t link = { 0 };
  link.dialect = dialect;
  link.bus = virtual_mavlink_bus_create();
  if (link.bus == NULL) {
    return link;
  }

  mavlink_link_t *gcs_link = virtual_mavlink_bus_create_endpoint(link.bus);
  mavlink_link_t *drone_link = virtual_mavlink_bus_create_endpoint(link.bus);
  if (gcs_link == NULL || drone_link == NULL) {
    virtual_mavlink_bus_close_all(link.bus);
    link.bus = NULL;
    return link;
  }

  link.gcs = mavlink_session_create(
    dialect,
    gcs_link,
    GCS_SYSTEM_ID,
    GCS_COMPONENT_ID,
    MAVLINK_VERSION_V2
  );
  link.drone = mavlink_session_create(
    dialect,
    drone_link,
    DRONE_SYSTEM_ID,
    DRONE_COMPONENT_ID,
    MAVLINK_VERSION_V2
  );

  if (link.gcs == NULL || link.drone == NULL) {
    virtual_mavlink_link_close(&link);
    link.bus = NULL;
  }
  return link;
}

void virtual_mavlink_link_close(virtual_mavlink_link_t *link) {
  if (link == NULL) {
    return;
  }
  if (link->gcs != NULL) {
    mavlink_session_close(link->gcs);
    link->gcs = NULL;
  }
  if (link->drone != NULL) {
    mavlink_session_close(link->drone);
    link->drone = NULL;
  }
  if (link->bus != NULL) {
    virtual_mavlink_bus_close_all(link->bus);
    link->bus = NULL;
  }
}
