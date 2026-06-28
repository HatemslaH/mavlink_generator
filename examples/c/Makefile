# SITL GCS example — links generated MAVLink C bindings + protocol layer.

GENERATED := ../../generated/c
CC      ?= gcc
CFLAGS  ?= -std=c11 -Wall -Wextra -O2
CFLAGS  += -I$(GENERATED) -Iinclude
LDFLAGS ?=

ifeq ($(OS),Windows_NT)
  TARGET := sitl_gcs.exe
else
  TARGET := sitl_gcs
  LDFLAGS += -lpthread
endif

PROTOCOL_SRCS := \
  $(GENERATED)/protocols/mavlink_link.c \
  $(GENERATED)/protocols/mavlink_cancellation.c \
  $(GENERATED)/protocols/mavlink_session.c \
  $(GENERATED)/protocols/param_codec.c \
  $(GENERATED)/protocols/heartbeat_protocol.c \
  $(GENERATED)/protocols/parameter_protocol.c \
  $(GENERATED)/protocols/mission_protocol.c \
  $(GENERATED)/protocols/command_protocol.c \
  $(GENERATED)/protocols/mavlink_vehicle_client.c

APP_SRCS := \
  src/sitl_gcs.c \
  src/serial_link.c \
  src/port_picker.c \
  src/sample_mission.c

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(APP_SRCS) $(PROTOCOL_SRCS)
	$(CC) $(CFLAGS) -o $@ $(APP_SRCS) $(PROTOCOL_SRCS) $(LDFLAGS)

clean:
	rm -f sitl_gcs sitl_gcs.exe
