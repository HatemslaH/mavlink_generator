#include "mavlink_link.h"

#include <stdlib.h>
#include <string.h>

typedef struct virtual_mavlink_endpoint {
  virtual_mavlink_bus_t *bus;
  mavlink_link_t link;
  mavlink_link_on_receive_fn on_receive;
  void *on_receive_ctx;
  int closed;
  struct virtual_mavlink_endpoint *next;
} virtual_mavlink_endpoint_t;

struct virtual_mavlink_bus {
  virtual_mavlink_endpoint_t *endpoints;
};

static void virtual_endpoint_set_on_receive(
  mavlink_link_t *link,
  mavlink_link_on_receive_fn cb,
  void *ctx
) {
  virtual_mavlink_endpoint_t *endpoint = (virtual_mavlink_endpoint_t *)link->impl;
  endpoint->on_receive = cb;
  endpoint->on_receive_ctx = ctx;
}

static int virtual_endpoint_send(mavlink_link_t *link, const uint8_t *data, size_t len) {
  virtual_mavlink_endpoint_t *sender = (virtual_mavlink_endpoint_t *)link->impl;
  if (sender == NULL || sender->closed) {
    return -1;
  }

  for (virtual_mavlink_endpoint_t *endpoint = sender->bus->endpoints; endpoint != NULL;
       endpoint = endpoint->next) {
    if (endpoint == sender || endpoint->closed) {
      continue;
    }
    if (endpoint->on_receive != NULL) {
      endpoint->on_receive(endpoint->on_receive_ctx, data, len);
    }
  }
  return 0;
}

static void virtual_endpoint_close(mavlink_link_t *link) {
  virtual_mavlink_endpoint_t *endpoint = (virtual_mavlink_endpoint_t *)link->impl;
  if (endpoint == NULL || endpoint->closed) {
    return;
  }
  endpoint->closed = 1;
  endpoint->on_receive = NULL;
  endpoint->on_receive_ctx = NULL;
}

virtual_mavlink_bus_t *virtual_mavlink_bus_create(void) {
  virtual_mavlink_bus_t *bus = (virtual_mavlink_bus_t *)calloc(1, sizeof(*bus));
  return bus;
}

mavlink_link_t *virtual_mavlink_bus_create_endpoint(virtual_mavlink_bus_t *bus) {
  if (bus == NULL) {
    return NULL;
  }

  virtual_mavlink_endpoint_t *endpoint =
    (virtual_mavlink_endpoint_t *)calloc(1, sizeof(*endpoint));
  if (endpoint == NULL) {
    return NULL;
  }

  endpoint->bus = bus;
  endpoint->link.send = virtual_endpoint_send;
  endpoint->link.set_on_receive = virtual_endpoint_set_on_receive;
  endpoint->link.close = virtual_endpoint_close;
  endpoint->link.impl = endpoint;
  endpoint->next = bus->endpoints;
  bus->endpoints = endpoint;
  return &endpoint->link;
}

void virtual_mavlink_bus_close_all(virtual_mavlink_bus_t *bus) {
  if (bus == NULL) {
    return;
  }

  while (bus->endpoints != NULL) {
    virtual_mavlink_endpoint_t *endpoint = bus->endpoints;
    bus->endpoints = endpoint->next;
    virtual_endpoint_close(&endpoint->link);
    free(endpoint);
  }
  free(bus);
}
