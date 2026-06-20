#ifndef MAVLINK_PROTOCOLS_MAVLINK_LINK_H
#define MAVLINK_PROTOCOLS_MAVLINK_LINK_H

#include <stddef.h>
#include <stdint.h>

/// Incoming raw bytes from the remote peer.
typedef void (*mavlink_link_on_receive_fn)(void *ctx, const uint8_t *data, size_t len);

/// Transport-agnostic MAVLink byte stream.
typedef struct mavlink_link {
  /// Send raw MAVLink frame bytes. Returns 0 on success.
  int (*send)(struct mavlink_link *link, const uint8_t *data, size_t len);
  /// Register handler for incoming bytes (called by the link implementation).
  void (*set_on_receive)(struct mavlink_link *link, mavlink_link_on_receive_fn cb, void *ctx);
  /// Release link resources.
  void (*close)(struct mavlink_link *link);
  void *impl;
} mavlink_link_t;

/// In-memory loopback for tests and virtual examples.
typedef struct virtual_mavlink_bus virtual_mavlink_bus_t;

/// Create a new endpoint on this bus.
mavlink_link_t *virtual_mavlink_bus_create_endpoint(virtual_mavlink_bus_t *bus);

/// Allocate a new virtual bus.
virtual_mavlink_bus_t *virtual_mavlink_bus_create(void);

/// Close every endpoint on the bus and free the bus.
void virtual_mavlink_bus_close_all(virtual_mavlink_bus_t *bus);

#endif
