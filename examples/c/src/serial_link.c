#include "serial_link.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <termios.h>
#include <unistd.h>
#endif

#define SERIAL_READ_BUF_SIZE 4096

typedef struct serial_mavlink_link_impl {
  mavlink_link_t link;
  mavlink_link_on_receive_fn on_receive;
  void *on_receive_ctx;
  volatile int closed;
#ifdef _WIN32
  HANDLE port;
  HANDLE thread;
#else
  int fd;
  pthread_t thread;
#endif
} serial_mavlink_link_impl_t;

static void serial_link_set_on_receive(mavlink_link_t *link, mavlink_link_on_receive_fn cb, void *ctx) {
  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)link->impl;
  impl->on_receive = cb;
  impl->on_receive_ctx = ctx;
}

static int serial_link_send(mavlink_link_t *link, const uint8_t *data, size_t len) {
  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)link->impl;
  if (impl == NULL || impl->closed) {
    return -1;
  }

#ifdef _WIN32
  DWORD written = 0;
  if (!WriteFile(impl->port, data, (DWORD)len, &written, NULL) || written != len) {
    return -1;
  }
  return 0;
#else
  size_t offset = 0;
  while (offset < len) {
    ssize_t n = write(impl->fd, data + offset, len - offset);
    if (n < 0) {
      if (errno == EINTR) {
        continue;
      }
      return -1;
    }
    offset += (size_t)n;
  }
  return 0;
#endif
}

#ifdef _WIN32

static DWORD WINAPI serial_read_thread(LPVOID param) {
  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)param;
  uint8_t buffer[SERIAL_READ_BUF_SIZE];

  while (!impl->closed) {
    DWORD nbytes = 0;
    if (!ReadFile(impl->port, buffer, SERIAL_READ_BUF_SIZE, &nbytes, NULL)) {
      break;
    }
    if (nbytes > 0 && impl->on_receive != NULL) {
      impl->on_receive(impl->on_receive_ctx, buffer, nbytes);
    }
  }
  return 0;
}

static int serial_open_port(serial_mavlink_link_impl_t *impl, const char *port_name, int baud_rate) {
  char path[64];
  if (strncmp(port_name, "\\\\.\\", 4) == 0) {
    snprintf(path, sizeof(path), "%s", port_name);
  } else {
    snprintf(path, sizeof(path), "\\\\.\\%s", port_name);
  }

  impl->port = CreateFileA(
    path,
    GENERIC_READ | GENERIC_WRITE,
    0,
    NULL,
    OPEN_EXISTING,
    0,
    NULL);
  if (impl->port == INVALID_HANDLE_VALUE) {
    return -1;
  }

  DCB dcb;
  memset(&dcb, 0, sizeof(dcb));
  dcb.DCBlength = sizeof(dcb);
  if (!GetCommState(impl->port, &dcb)) {
    CloseHandle(impl->port);
    impl->port = INVALID_HANDLE_VALUE;
    return -1;
  }

  dcb.BaudRate = (DWORD)baud_rate;
  dcb.ByteSize = 8;
  dcb.Parity = NOPARITY;
  dcb.StopBits = ONESTOPBIT;
  dcb.fDtrControl = DTR_CONTROL_ENABLE;
  dcb.fRtsControl = RTS_CONTROL_ENABLE;
  if (!SetCommState(impl->port, &dcb)) {
    CloseHandle(impl->port);
    impl->port = INVALID_HANDLE_VALUE;
    return -1;
  }

  COMMTIMEOUTS timeouts = { 0 };
  timeouts.ReadIntervalTimeout = 50;
  timeouts.ReadTotalTimeoutConstant = 50;
  timeouts.ReadTotalTimeoutMultiplier = 0;
  SetCommTimeouts(impl->port, &timeouts);
  return 0;
}

static void serial_close_port(serial_mavlink_link_impl_t *impl) {
  if (impl->port != INVALID_HANDLE_VALUE) {
    CloseHandle(impl->port);
    impl->port = INVALID_HANDLE_VALUE;
  }
}

#else

static speed_t serial_baud_to_speed(int baud_rate) {
  switch (baud_rate) {
  case 9600: return B9600;
  case 19200: return B19200;
  case 38400: return B38400;
  case 57600: return B57600;
  case 115200: return B115200;
  case 230400: return B230400;
  case 460800: return B460800;
  case 921600: return B921600;
  default: return B57600;
  }
}

static int serial_open_port(serial_mavlink_link_impl_t *impl, const char *port_name, int baud_rate) {
  impl->fd = open(port_name, O_RDWR | O_NOCTTY | O_NONBLOCK);
  if (impl->fd < 0) {
    return -1;
  }

  struct termios tty;
  if (tcgetattr(impl->fd, &tty) != 0) {
    close(impl->fd);
    impl->fd = -1;
    return -1;
  }

  cfmakeraw(&tty);
  cfsetispeed(&tty, serial_baud_to_speed(baud_rate));
  cfsetospeed(&tty, serial_baud_to_speed(baud_rate));
  tty.c_cflag |= (CLOCAL | CREAD);
  tty.c_cflag &= ~CSIZE;
  tty.c_cflag |= CS8;
  tty.c_cflag &= ~PARENB;
  tty.c_cflag &= ~CSTOPB;
  tty.c_cc[VMIN] = 0;
  tty.c_cc[VTIME] = 5;

  if (tcsetattr(impl->fd, TCSANOW, &tty) != 0) {
    close(impl->fd);
    impl->fd = -1;
    return -1;
  }

  int flags = fcntl(impl->fd, F_GETFL, 0);
  if (flags >= 0) {
    fcntl(impl->fd, F_SETFL, flags & ~O_NONBLOCK);
  }
  return 0;
}

static void *serial_read_thread(void *param) {
  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)param;
  uint8_t buffer[SERIAL_READ_BUF_SIZE];

  while (!impl->closed) {
    ssize_t nbytes = read(impl->fd, buffer, SERIAL_READ_BUF_SIZE);
    if (nbytes < 0) {
      if (errno == EINTR) {
        continue;
      }
      break;
    }
    if (nbytes == 0) {
      usleep(10000);
      continue;
    }
    if (impl->on_receive != NULL) {
      impl->on_receive(impl->on_receive_ctx, buffer, (size_t)nbytes);
    }
  }
  return NULL;
}

static void serial_close_port(serial_mavlink_link_impl_t *impl) {
  if (impl->fd >= 0) {
    close(impl->fd);
    impl->fd = -1;
  }
}

#endif

static void serial_link_close(mavlink_link_t *link) {
  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)link->impl;
  if (impl == NULL || impl->closed) {
    return;
  }

  impl->closed = 1;
  serial_close_port(impl);
#ifdef _WIN32
  if (impl->thread != NULL) {
    WaitForSingleObject(impl->thread, INFINITE);
    CloseHandle(impl->thread);
    impl->thread = NULL;
  }
#else
  pthread_join(impl->thread, NULL);
#endif
  impl->on_receive = NULL;
  impl->on_receive_ctx = NULL;
}

mavlink_link_t *serial_mavlink_link_open(const char *port_name, int baud_rate) {
  if (port_name == NULL) {
    return NULL;
  }

  serial_mavlink_link_impl_t *impl = (serial_mavlink_link_impl_t *)calloc(1, sizeof(*impl));
  if (impl == NULL) {
    return NULL;
  }

#ifdef _WIN32
  impl->port = INVALID_HANDLE_VALUE;
#endif

  if (serial_open_port(impl, port_name, baud_rate) != 0) {
    free(impl);
    return NULL;
  }

  impl->link.send = serial_link_send;
  impl->link.set_on_receive = serial_link_set_on_receive;
  impl->link.close = serial_link_close;
  impl->link.impl = impl;

#ifdef _WIN32
  impl->thread = CreateThread(NULL, 0, serial_read_thread, impl, 0, NULL);
  if (impl->thread == NULL) {
    serial_close_port(impl);
    free(impl);
    return NULL;
  }
#else
  if (pthread_create(&impl->thread, NULL, serial_read_thread, impl) != 0) {
    serial_close_port(impl);
    free(impl);
    return NULL;
  }
#endif

  return &impl->link;
}

void serial_mavlink_link_close(mavlink_link_t *link) {
  if (link == NULL || link->close == NULL) {
    return;
  }
  link->close(link);
  free(link->impl);
}
