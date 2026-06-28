#ifndef MAVLINK_SITL_PORT_PICKER_H
#define MAVLINK_SITL_PORT_PICKER_H

/// List serial ports and read a selection from stdin. Returns heap-allocated port name.
char *pick_serial_port(void);

/// Parse `--baud <rate>` from argv (default 57600).
int parse_baud_rate(int argc, char **argv, int default_baud);

#endif
