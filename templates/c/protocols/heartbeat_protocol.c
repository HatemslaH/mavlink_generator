#include "heartbeat_protocol.h"

#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#include <time.h>
#include <unistd.h>
#endif

#define MAVLINK_HEARTBEAT_MSG_ID 0
#define MAVLINK_HEARTBEAT_MAX_TRACKED 32
#define MAVLINK_HEARTBEAT_PAYLOAD_MAX 32

struct heartbeat_monitor {
  mavlink_session_t *session;
  int timeout_ms;
  int running;
  tracked_heartbeat_t states[MAVLINK_HEARTBEAT_MAX_TRACKED];
  int state_count;
  mavlink_message_subscription_t *subscription;
};

struct heartbeat_publisher {
  mavlink_session_t *session;
  uint8_t payload[MAVLINK_HEARTBEAT_PAYLOAD_MAX];
  size_t payload_len;
  uint32_t message_id;
  uint8_t crc_extra;
  int interval_ms;
  int running;
#ifdef _WIN32
  HANDLE thread;
  CRITICAL_SECTION lock;
#else
  pthread_t thread;
  int thread_running;
  pthread_mutex_t lock;
#endif
};

static uint64_t heartbeat_now_ms(void) {
#ifdef _WIN32
  return (uint64_t)GetTickCount64();
#else
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (uint64_t)ts.tv_sec * 1000ULL + (uint64_t)ts.tv_nsec / 1000000ULL;
#endif
}

static int heartbeat_node_index(const heartbeat_monitor_t *monitor, mavlink_node_t node) {
  for (int i = 0; i < monitor->state_count; i++) {
    if (monitor->states[i].node.system_id == node.system_id &&
        monitor->states[i].node.component_id == node.component_id) {
      return i;
    }
  }
  return -1;
}

static void heartbeat_monitor_on_frame(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  heartbeat_monitor_t *monitor = (heartbeat_monitor_t *)user_data;
  if (monitor == NULL || !monitor->running || parsed_message == NULL) {
    return;
  }
  if (frame->message_id != MAVLINK_HEARTBEAT_MSG_ID) {
    return;
  }

  mavlink_node_t node = { frame->system_id, frame->component_id };
  int index = heartbeat_node_index(monitor, node);
  if (index < 0) {
    if (monitor->state_count >= MAVLINK_HEARTBEAT_MAX_TRACKED) {
      return;
    }
    index = monitor->state_count++;
    monitor->states[index].node = node;
    monitor->states[index].heartbeat = malloc(MAVLINK_HEARTBEAT_PAYLOAD_MAX);
  }

  tracked_heartbeat_t *state = &monitor->states[index];
  if (state->heartbeat != NULL) {
    memcpy(state->heartbeat, parsed_message, MAVLINK_HEARTBEAT_PAYLOAD_MAX);
  }
  state->received_at_ms = heartbeat_now_ms();
  state->online = true;
}

heartbeat_monitor_t *heartbeat_monitor_create(mavlink_session_t *session, int timeout_ms) {
  heartbeat_monitor_t *monitor = (heartbeat_monitor_t *)calloc(1, sizeof(*monitor));
  if (monitor == NULL) {
    return NULL;
  }
  monitor->session = session;
  monitor->timeout_ms = timeout_ms > 0 ? timeout_ms : 5000;
  return monitor;
}

void heartbeat_monitor_start(heartbeat_monitor_t *monitor) {
  if (monitor == NULL || monitor->running) {
    return;
  }
  monitor->running = 1;
  monitor->subscription = mavlink_session_listen_message(
    monitor->session,
    MAVLINK_HEARTBEAT_MSG_ID,
    0,
    0,
    heartbeat_monitor_on_frame,
    monitor
  );
}

void heartbeat_monitor_stop(heartbeat_monitor_t *monitor) {
  if (monitor == NULL || !monitor->running) {
    return;
  }
  monitor->running = 0;
  if (monitor->subscription != NULL) {
    mavlink_message_subscription_cancel(monitor->subscription);
    monitor->subscription = NULL;
  }
}

const tracked_heartbeat_t *heartbeat_monitor_state_for(
  const heartbeat_monitor_t *monitor,
  mavlink_node_t node
) {
  if (monitor == NULL) {
    return NULL;
  }
  int index = heartbeat_node_index(monitor, node);
  return index >= 0 ? &monitor->states[index] : NULL;
}

bool heartbeat_monitor_is_online(const heartbeat_monitor_t *monitor, mavlink_node_t node) {
  const tracked_heartbeat_t *state = heartbeat_monitor_state_for(monitor, node);
  return state != NULL && state->online;
}

bool heartbeat_monitor_is_online_ids(
  const heartbeat_monitor_t *monitor,
  uint8_t system_id,
  uint8_t component_id
) {
  mavlink_node_t node = { system_id, component_id };
  return heartbeat_monitor_is_online(monitor, node);
}

static bool heartbeat_should_exclude(uint8_t system_id, const uint8_t *exclude, size_t count) {
  for (size_t i = 0; i < count; i++) {
    if (exclude[i] == system_id) {
      return true;
    }
  }
  return false;
}

mavlink_wait_result_t heartbeat_monitor_wait_for_vehicle(
  heartbeat_monitor_t *monitor,
  const uint8_t *exclude_system_ids,
  size_t exclude_count,
  int timeout_ms,
  mavlink_cancellation_token_t *cancel,
  mavlink_node_t *out_vehicle
) {
  if (monitor == NULL) {
    return MAVLINK_WAIT_ERROR;
  }

  for (int i = 0; i < monitor->state_count; i++) {
    if (monitor->states[i].online &&
        !heartbeat_should_exclude(monitor->states[i].node.system_id, exclude_system_ids, exclude_count)) {
      if (out_vehicle != NULL) {
        *out_vehicle = monitor->states[i].node;
      }
      return MAVLINK_WAIT_OK;
    }
  }

  uint64_t deadline = heartbeat_now_ms() + (uint64_t)(timeout_ms > 0 ? timeout_ms : 60000);
  while (heartbeat_now_ms() < deadline) {
    if (mavlink_cancellation_token_is_cancelled(cancel)) {
      return MAVLINK_WAIT_CANCELLED;
    }
    for (int i = 0; i < monitor->state_count; i++) {
      if (monitor->states[i].online &&
          !heartbeat_should_exclude(monitor->states[i].node.system_id, exclude_system_ids, exclude_count)) {
        if (out_vehicle != NULL) {
          *out_vehicle = monitor->states[i].node;
        }
        return MAVLINK_WAIT_OK;
      }
    }
#ifdef _WIN32
    Sleep(10);
#else
    usleep(10000);
#endif
  }
  return MAVLINK_WAIT_TIMEOUT;
}

void heartbeat_monitor_destroy(heartbeat_monitor_t *monitor) {
  if (monitor == NULL) {
    return;
  }
  heartbeat_monitor_stop(monitor);
  for (int i = 0; i < monitor->state_count; i++) {
    free(monitor->states[i].heartbeat);
  }
  free(monitor);
}

#ifdef _WIN32
static DWORD WINAPI heartbeat_publisher_thread(LPVOID arg) {
#else
static void *heartbeat_publisher_thread(void *arg) {
#endif
  heartbeat_publisher_t *publisher = (heartbeat_publisher_t *)arg;
  while (publisher->running) {
    heartbeat_publisher_send_once(publisher);
#ifdef _WIN32
    Sleep((DWORD)publisher->interval_ms);
#else
    usleep((unsigned int)publisher->interval_ms * 1000U);
#endif
  }
#ifdef _WIN32
  return 0;
#else
  return NULL;
#endif
}

heartbeat_publisher_t *heartbeat_publisher_create(
  mavlink_session_t *session,
  const void *heartbeat,
  uint32_t message_id,
  uint8_t crc_extra,
  size_t encoded_length,
  int interval_ms
) {
  heartbeat_publisher_t *publisher = (heartbeat_publisher_t *)calloc(1, sizeof(*publisher));
  if (publisher == NULL) {
    return NULL;
  }
  publisher->session = session;
  publisher->message_id = message_id;
  publisher->crc_extra = crc_extra;
  publisher->interval_ms = interval_ms > 0 ? interval_ms : 1000;
  publisher->payload_len = encoded_length < MAVLINK_HEARTBEAT_PAYLOAD_MAX
    ? encoded_length
    : MAVLINK_HEARTBEAT_PAYLOAD_MAX;
  if (heartbeat != NULL && publisher->payload_len > 0) {
    memcpy(publisher->payload, heartbeat, publisher->payload_len);
  }
#ifdef _WIN32
  InitializeCriticalSection(&publisher->lock);
#endif
  return publisher;
}

void heartbeat_publisher_start(heartbeat_publisher_t *publisher) {
  if (publisher == NULL || publisher->running) {
    return;
  }
  publisher->running = 1;
  heartbeat_publisher_send_once(publisher);
#ifdef _WIN32
  publisher->thread = CreateThread(NULL, 0, heartbeat_publisher_thread, publisher, 0, NULL);
#else
  pthread_create(&publisher->thread, NULL, heartbeat_publisher_thread, publisher);
#endif
}

void heartbeat_publisher_stop(heartbeat_publisher_t *publisher) {
  if (publisher == NULL || !publisher->running) {
    return;
  }
  publisher->running = 0;
#ifdef _WIN32
  if (publisher->thread != NULL) {
    WaitForSingleObject(publisher->thread, INFINITE);
    CloseHandle(publisher->thread);
    publisher->thread = NULL;
  }
  DeleteCriticalSection(&publisher->lock);
#else
  pthread_join(publisher->thread, NULL);
  pthread_mutex_destroy(&publisher->lock);
#endif
}

int heartbeat_publisher_send_once(heartbeat_publisher_t *publisher) {
  if (publisher == NULL || publisher->session == NULL) {
    return -1;
  }
  return mavlink_session_send(
    publisher->session,
    publisher->message_id,
    publisher->crc_extra,
    publisher->payload,
    publisher->payload_len
  );
}

void heartbeat_publisher_update_heartbeat(heartbeat_publisher_t *publisher, const void *heartbeat) {
  if (publisher == NULL || heartbeat == NULL) {
    return;
  }
  memcpy(publisher->payload, heartbeat, publisher->payload_len);
}

void heartbeat_publisher_destroy(heartbeat_publisher_t *publisher) {
  if (publisher == NULL) {
    return;
  }
  heartbeat_publisher_stop(publisher);
  free(publisher);
}
