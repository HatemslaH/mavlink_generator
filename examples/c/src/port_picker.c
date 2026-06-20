#include "port_picker.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <dirent.h>
#include <errno.h>
#include <sys/stat.h>
#endif

#define MAX_PORTS 64
#define PORT_NAME_MAX 64

typedef struct {
  char names[MAX_PORTS][PORT_NAME_MAX];
  int count;
} serial_port_list_t;

#ifdef _WIN32

static void serial_port_list_add(serial_port_list_t *list, const char *name) {
  if (list->count >= MAX_PORTS || name == NULL || name[0] == '\0') {
    return;
  }
  for (int i = 0; i < list->count; i++) {
    if (strcmp(list->names[i], name) == 0) {
      return;
    }
  }
  snprintf(list->names[list->count], PORT_NAME_MAX, "%s", name);
  list->count++;
}

static void serial_port_list_enumerate(serial_port_list_t *list) {
  HKEY key = NULL;
  if (RegOpenKeyExA(
        HKEY_LOCAL_MACHINE,
        "HARDWARE\\DEVICEMAP\\SERIALCOMM",
        0,
        KEY_READ,
        &key) != ERROR_SUCCESS) {
    return;
  }

  DWORD index = 0;
  char value_name[256];
  char data[PORT_NAME_MAX];
  while (list->count < MAX_PORTS) {
    DWORD value_name_len = (DWORD)sizeof(value_name);
    DWORD data_len = (DWORD)sizeof(data);
    DWORD type = 0;
    LONG result = RegEnumValueA(key, index++, value_name, &value_name_len, NULL, &type, (LPBYTE)data, &data_len);
    if (result == ERROR_NO_MORE_ITEMS) {
      break;
    }
    if (result != ERROR_SUCCESS || type != REG_SZ) {
      continue;
    }
    serial_port_list_add(list, data);
  }

  RegCloseKey(key);
}

#else

static int serial_port_name_matches(const char *name) {
  return strncmp(name, "ttyACM", 6) == 0 ||
         strncmp(name, "ttyUSB", 6) == 0 ||
         strncmp(name, "cu.usb", 6) == 0 ||
         strncmp(name, "tty.usb", 7) == 0;
}

static void serial_port_list_add_posix(serial_port_list_t *list, const char *dev_path) {
  if (list->count >= MAX_PORTS) {
    return;
  }
  snprintf(list->names[list->count], PORT_NAME_MAX, "%s", dev_path);
  list->count++;
}

static void serial_port_list_enumerate(serial_port_list_t *list) {
  DIR *dir = opendir("/dev");
  if (dir == NULL) {
    return;
  }

  struct dirent *entry;
  while ((entry = readdir(dir)) != NULL && list->count < MAX_PORTS) {
    if (!serial_port_name_matches(entry->d_name)) {
      continue;
    }
    char path[PORT_NAME_MAX];
    snprintf(path, sizeof(path), "/dev/%s", entry->d_name);
    serial_port_list_add_posix(list, path);
  }

  closedir(dir);
}

#endif

int parse_baud_rate(int argc, char **argv, int default_baud) {
  for (int i = 1; i < argc - 1; i++) {
    if (strcmp(argv[i], "--baud") == 0) {
      char *end = NULL;
      long value = strtol(argv[i + 1], &end, 10);
      if (end == argv[i + 1] || value <= 0) {
        fprintf(stderr, "Invalid --baud value: %s\n", argv[i + 1]);
        exit(EXIT_FAILURE);
      }
      return (int)value;
    }
  }
  return default_baud;
}

char *pick_serial_port(void) {
  serial_port_list_t list = { 0 };
  serial_port_list_enumerate(&list);

  if (list.count == 0) {
    fprintf(stderr, "No serial ports found. Connect SITL or a USB adapter.\n");
    return NULL;
  }

  printf("\nAvailable serial ports:\n");
  for (int i = 0; i < list.count; i++) {
    printf("  [%d] %s\n", i, list.names[i]);
  }

  printf("\nSelect port [0-%d]: ", list.count - 1);
  fflush(stdout);

  char line[64];
  if (fgets(line, sizeof(line), stdin) == NULL) {
    fprintf(stderr, "Port selection required.\n");
    return NULL;
  }

  line[strcspn(line, "\r\n")] = '\0';
  if (line[0] == '\0') {
    fprintf(stderr, "Port selection required.\n");
    return NULL;
  }

  char *end = NULL;
  long selected = strtol(line, &end, 10);
  if (end == line || selected < 0 || selected >= list.count) {
    fprintf(stderr, "Invalid port selection: %s\n", line);
    return NULL;
  }

  char *port_name = strdup(list.names[selected]);
  if (port_name != NULL) {
    printf("Selected %s\n", port_name);
  }
  return port_name;
}
