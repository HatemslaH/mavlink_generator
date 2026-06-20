#ifndef MAVLINK_SITL_SERIAL_LINK_H
#define MAVLINK_SITL_SERIAL_LINK_H

#include "protocols/mavlink_link.h"

/// Open [port_name] at [baud_rate] (MAVLink SITL commonly uses 57600 or 115200).
mavlink_link_t *serial_mavlink_link_open(const char *port_name, int baud_rate);

/// Release the serial port and reader thread.
void serial_mavlink_link_close(mavlink_link_t *link);

#endif
