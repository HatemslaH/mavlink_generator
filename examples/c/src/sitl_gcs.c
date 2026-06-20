#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <strings.h>
#include <time.h>
#include <unistd.h>
#endif

#include "gcs_context.h"
#include "port_picker.h"
#include "sample_mission.h"
#include "serial_link.h"

static void sleep_ms(int ms) {
#ifdef _WIN32
  Sleep((DWORD)ms);
#else
  usleep((unsigned int)ms * 1000U);
#endif
}

static uint64_t now_ms(void) {
#ifdef _WIN32
  return (uint64_t)GetTickCount64();
#else
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (uint64_t)ts.tv_sec * 1000ULL + (uint64_t)ts.tv_nsec / 1000000ULL;
#endif
}

static const char *wait_result_string(mavlink_wait_result_t result) {
  switch (result) {
  case MAVLINK_WAIT_OK: return "ok";
  case MAVLINK_WAIT_TIMEOUT: return "timeout";
  case MAVLINK_WAIT_CANCELLED: return "cancelled";
  case MAVLINK_WAIT_CLOSED: return "closed";
  default: return "error";
  }
}

static void print_help(void) {
  printf("Commands:\n");
  printf("  help              Show this help\n");
  printf("  hb                Heartbeat / link status\n");
  printf("  cancel            Cancel in-flight params/mission operation\n");
  printf("  params            Request full parameter list (with progress)\n");
  printf("  pr <name>         Read one parameter by name\n");
  printf("  pw <name> <value> Write parameter (type from cache or REAL32)\n");
  printf("  mu                Upload hardcoded sample mission\n");
  printf("  md                Download mission from vehicle\n");
  printf("  mc                Clear onboard mission\n");
  printf("  ms <seq>          Set active mission item (mission + command)\n");
  printf("  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)\n");
  printf("  si <msgId> <us>   Set message interval (microseconds)\n");
  printf("  att [seconds]     Stream ATTITUDE via listen_message (default 5 s)\n");
  printf("  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)\n");
  printf("  disarm [force]    Disarm motors\n");
  printf("  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH\n");
  printf("  quit              Exit\n");
}

static void cancel_operation(gcs_context_t *ctx) {
  if (ctx->operation_cancel == NULL || mavlink_cancellation_token_is_cancelled(ctx->operation_cancel)) {
    printf("[cancel] no active cancellable operation\n");
    return;
  }
  mavlink_cancellation_token_cancel(ctx->operation_cancel);
  printf("[cancel] signalled\n");
}

static void print_heartbeat_status(const gcs_context_t *ctx) {
  heartbeat_monitor_t *monitor = gcs_context_heartbeat_monitor(ctx);
  const tracked_heartbeat_t *state = heartbeat_monitor_state_for(monitor, ctx->vehicle);
  int online = heartbeat_monitor_is_online(monitor, ctx->vehicle);

  printf(
    "[heartbeat] vehicle sys=%u comp=%u online=%d\n",
    ctx->vehicle.system_id,
    ctx->vehicle.component_id,
    online
  );
  if (state != NULL && state->heartbeat != NULL) {
    const heartbeat_t *hb = (const heartbeat_t *)state->heartbeat;
    uint64_t age_ms = now_ms() - state->received_at_ms;
    printf(
      "  last=%llums ago type=%d status=%d\n",
      (unsigned long long)age_ms,
      (int)hb->type,
      (int)hb->system_status
    );
  } else {
    printf("  no heartbeat received yet\n");
  }
}

static void on_param_progress(const param_entry_t *entry, int received, int expected, void *user_data) {
  (void)user_data;
  if (received == 1) {
    printf("[parameters] expecting %d parameters\n", expected);
  }
  printf(
    "[parameters] %d/%d %s=%g (type=%d)\n",
    received,
    expected,
    entry->id,
    entry->value,
    (int)entry->type
  );
}

static int fetch_all_parameters(gcs_context_t *ctx) {
  if (ctx->operation_cancel != NULL) {
    mavlink_cancellation_token_dispose(ctx->operation_cancel);
  }
  ctx->operation_cancel = mavlink_cancellation_token_create();

  printf("[parameters] waiting for PARAM_VALUE stream...\n");
  size_t count = 0;
  mavlink_wait_result_t result = parameter_protocol_fetch_all(
    gcs_context_parameters(ctx),
    NULL,
    0,
    &count,
    on_param_progress,
    NULL,
    ctx->operation_cancel
  );
  if (result == MAVLINK_WAIT_CANCELLED) {
    printf("Operation cancelled.\n");
    return -1;
  }
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: parameter fetch %s\n", wait_result_string(result));
    return -1;
  }
  printf("[parameters] complete (%zu total)\n", count);
  return 0;
}

static double parse_param_value(const char *raw, mavlink_param_type_t type) {
  switch (type) {
  case MAV_PARAM_TYPE_INT8:
  case MAV_PARAM_TYPE_INT16:
  case MAV_PARAM_TYPE_INT32:
  case MAV_PARAM_TYPE_UINT8:
  case MAV_PARAM_TYPE_UINT16:
  case MAV_PARAM_TYPE_UINT32:
    return (double)strtol(raw, NULL, 10);
  default:
    return strtod(raw, NULL);
  }
}

static int read_parameter(gcs_context_t *ctx, const char *name) {
  printf("[parameters] reading %s...\n", name);
  param_entry_t entry;
  mavlink_wait_result_t result =
    parameter_protocol_read_by_name(gcs_context_parameters(ctx), name, &entry, NULL);
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: parameter read %s\n", wait_result_string(result));
    return -1;
  }
  printf(
    "[parameters] %s=%g (type=%d, index %u/%u)\n",
    name,
    entry.value,
    (int)entry.type,
    entry.index,
    entry.count
  );
  return 0;
}

static int write_parameter(gcs_context_t *ctx, const char *name, const char *raw_value) {
  mavlink_param_type_t parse_type =
    parameter_protocol_type_for_name(gcs_context_parameters(ctx), name);
  double value = parse_param_value(raw_value, parse_type);

  printf("[parameters] writing %s=%g (type=%d)...\n", name, value, (int)parse_type);
  param_entry_t entry;
  mavlink_wait_result_t result = parameter_protocol_write_by_name(
    gcs_context_parameters(ctx),
    name,
    value,
    0,
    &entry,
    NULL
  );
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: parameter write %s\n", wait_result_string(result));
    return -1;
  }
  printf("[parameters] ack %s=%g (type=%d)\n", name, entry.value, (int)entry.type);
  return 0;
}

static void on_mission_upload_progress(int sent, int total, const mission_item_int_t *item, void *user_data) {
  (void)user_data;
  char desc[160];
  describe_mission_item(item, desc, sizeof(desc));
  printf("[mission upload] %d/%d %s\n", sent, total, desc);
}

static int upload_mission(gcs_context_t *ctx) {
  mission_item_int_t plan[8];
  size_t plan_count = build_sample_mission(
    plan,
    sizeof(plan) / sizeof(plan[0]),
    gcs_context_target_system(ctx),
    gcs_context_target_component(ctx)
  );
  if (plan_count == 0) {
    printf("Error: failed to build sample mission\n");
    return -1;
  }

  if (ctx->operation_cancel != NULL) {
    mavlink_cancellation_token_dispose(ctx->operation_cancel);
  }
  ctx->operation_cancel = mavlink_cancellation_token_create();

  printf("[mission] uploading %zu hardcoded items...\n", plan_count);
  MAV_MISSION_RESULT upload_result;
  mavlink_wait_result_t result = mission_protocol_upload(
    gcs_context_mission(ctx),
    plan,
    plan_count,
    MAV_MISSION_TYPE_MISSION,
    on_mission_upload_progress,
    NULL,
    ctx->operation_cancel,
    &upload_result
  );
  if (result == MAVLINK_WAIT_CANCELLED) {
    printf("Operation cancelled.\n");
    return -1;
  }
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: mission upload %s\n", wait_result_string(result));
    return -1;
  }
  printf("[mission] upload finished: %d\n", (int)upload_result);
  return 0;
}

static void on_mission_download_progress(int received, int total, const mission_item_int_t *item, void *user_data) {
  (void)user_data;
  char desc[160];
  describe_mission_item(item, desc, sizeof(desc));
  printf("[mission download] %d/%d %s\n", received, total, desc);
}

static int download_mission(gcs_context_t *ctx) {
  if (ctx->operation_cancel != NULL) {
    mavlink_cancellation_token_dispose(ctx->operation_cancel);
  }
  ctx->operation_cancel = mavlink_cancellation_token_create();

  mission_item_int_t items[MAVLINK_MISSION_MAX_ITEMS];
  size_t count = 0;
  mavlink_wait_result_t result = mission_protocol_download(
    gcs_context_mission(ctx),
    items,
    MAVLINK_MISSION_MAX_ITEMS,
    &count,
    MAV_MISSION_TYPE_MISSION,
    on_mission_download_progress,
    NULL,
    ctx->operation_cancel
  );
  if (result == MAVLINK_WAIT_CANCELLED) {
    printf("Operation cancelled.\n");
    return -1;
  }
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: mission download %s\n", wait_result_string(result));
    return -1;
  }

  printf("[mission] on vehicle:\n");
  for (size_t i = 0; i < count; i++) {
    char desc[160];
    describe_mission_item(&items[i], desc, sizeof(desc));
    printf("  %s\n", desc);
  }
  return 0;
}

static int clear_mission(gcs_context_t *ctx) {
  printf("[mission] sending MISSION_CLEAR_ALL...\n");
  MAV_MISSION_RESULT clear_result;
  mavlink_wait_result_t result = mission_protocol_clear(
    gcs_context_mission(ctx),
    MAV_MISSION_TYPE_MISSION,
    NULL,
    &clear_result
  );
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: mission clear %s\n", wait_result_string(result));
    return -1;
  }
  printf("[mission] clear result: %d\n", (int)clear_result);
  return 0;
}

static int set_mission_current(gcs_context_t *ctx, int seq) {
  printf("[mission] set current seq=%d (mission + command)...\n", seq);
  mission_set_current_result_t result_data;
  mavlink_wait_result_t result = mission_protocol_set_current_with_command(
    gcs_context_mission(ctx),
    (uint16_t)seq,
    gcs_context_command(ctx),
    1,
    0,
    NULL,
    &result_data
  );
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: set mission current %s\n", wait_result_string(result));
    return -1;
  }
  printf(
    "[mission] seq=%u command ack=%s\n",
    result_data.sequence,
    result_data.has_command_ack ? "yes" : "n/a"
  );
  if (result_data.has_command_ack) {
    printf("  ack result=%d\n", (int)result_data.command_ack.result);
  }
  return 0;
}

static int request_message(gcs_context_t *ctx, int msg_id) {
  printf("[command] REQUEST_MESSAGE id=%d\n", msg_id);
  command_ack_t ack;
  mavlink_wait_result_t result = command_protocol_request_message(
    gcs_context_command(ctx),
    (uint32_t)msg_id,
    0.0f,
    0,
    NULL,
    &ack
  );
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: request message %s\n", wait_result_string(result));
    return -1;
  }
  printf("[command] ack: %d\n", (int)ack.result);

  if (msg_id == (int)attitude_MSG_ID) {
    printf("[telemetry] waiting for ATTITUDE...\n");
    attitude_t attitude;
    mavlink_frame_t frame;
    result = mavlink_session_wait_for_message_id(
      gcs_context_session(ctx),
      attitude_MSG_ID,
      gcs_context_target_system(ctx),
      0,
      5000,
      NULL,
      &frame,
      &attitude,
      sizeof(attitude)
    );
    if (result != MAVLINK_WAIT_OK) {
      printf("Error: wait for ATTITUDE %s\n", wait_result_string(result));
      return -1;
    }
    printf(
      "[telemetry] roll=%g pitch=%g yaw=%g\n",
      (double)attitude.roll,
      (double)attitude.pitch,
      (double)attitude.yaw
    );
  }
  return 0;
}

static int set_message_interval(gcs_context_t *ctx, int msg_id, int interval_us) {
  printf("[command] SET_MESSAGE_INTERVAL id=%d interval=%d us\n", msg_id, interval_us);
  command_ack_t ack;
  mavlink_wait_result_t result;
  if (interval_us == 0) {
    result = command_protocol_stop_message_interval(
      gcs_context_command(ctx),
      (uint32_t)msg_id,
      0,
      NULL,
      &ack
    );
  } else {
    result = command_protocol_set_message_interval(
      gcs_context_command(ctx),
      (uint32_t)msg_id,
      interval_us,
      0,
      NULL,
      &ack
    );
  }
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: set message interval %s\n", wait_result_string(result));
    return -1;
  }
  printf("[command] ack: %d\n", (int)ack.result);
  return 0;
}

typedef struct {
  int count;
} attitude_stream_ctx_t;

static void on_attitude_message(
  mavlink_session_t *session,
  const mavlink_frame_t *frame,
  void *parsed_message,
  void *user_data
) {
  (void)session;
  (void)frame;
  attitude_stream_ctx_t *ctx = (attitude_stream_ctx_t *)user_data;
  if (parsed_message == NULL || ctx == NULL) {
    return;
  }
  const attitude_t *attitude = (const attitude_t *)parsed_message;
  ctx->count++;
  printf(
    "[attitude] #%d roll=%.3f pitch=%.3f yaw=%.3f\n",
    ctx->count,
    (double)attitude->roll,
    (double)attitude->pitch,
    (double)attitude->yaw
  );
}

static int stream_attitude(gcs_context_t *ctx, int seconds) {
  printf("[telemetry] streaming ATTITUDE for %ds (subscribe + interval)...\n", seconds);

  command_ack_t ack;
  mavlink_wait_result_t result = command_protocol_set_message_interval(
    gcs_context_command(ctx),
    attitude_MSG_ID,
    100000,
    0,
    NULL,
    &ack
  );
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: set ATTITUDE interval %s\n", wait_result_string(result));
    return -1;
  }

  attitude_stream_ctx_t stream_ctx = { 0 };
  mavlink_message_subscription_t *subscription = mavlink_session_listen_message(
    gcs_context_session(ctx),
    attitude_MSG_ID,
    gcs_context_target_system(ctx),
    0,
    on_attitude_message,
    &stream_ctx
  );

  sleep_ms(seconds * 1000);

  mavlink_message_subscription_cancel(subscription);
  command_protocol_stop_message_interval(
    gcs_context_command(ctx),
    attitude_MSG_ID,
    0,
    NULL,
    &ack
  );
  printf("[telemetry] received %d ATTITUDE messages\n", stream_ctx.count);
  return 0;
}

static int arm_vehicle(gcs_context_t *ctx, int force) {
  printf("[command] ARM%s...\n", force ? " (force)" : "");
  command_ack_t ack;
  mavlink_wait_result_t result =
    command_protocol_arm(gcs_context_command(ctx), force, 0, NULL, &ack);
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: arm %s\n", wait_result_string(result));
    return -1;
  }
  printf("[command] ack: %d\n", (int)ack.result);
  return 0;
}

static int disarm_vehicle(gcs_context_t *ctx, int force) {
  printf("[command] DISARM%s...\n", force ? " (force)" : "");
  command_ack_t ack;
  mavlink_wait_result_t result =
    command_protocol_disarm(gcs_context_command(ctx), force, 0, NULL, &ack);
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: disarm %s\n", wait_result_string(result));
    return -1;
  }
  printf("[command] ack: %d\n", (int)ack.result);
  return 0;
}

static int return_to_launch(gcs_context_t *ctx) {
  printf("[command] RETURN_TO_LAUNCH...\n");
  command_ack_t ack;
  mavlink_wait_result_t result =
    command_protocol_return_to_launch(gcs_context_command(ctx), 0, NULL, &ack);
  if (result != MAVLINK_WAIT_OK) {
    printf("Error: RTL %s\n", wait_result_string(result));
    return -1;
  }
  printf("[command] ack: %d\n", (int)ack.result);
  return 0;
}

static void trim_line(char *line) {
  if (line == NULL) {
    return;
  }
  size_t len = strlen(line);
  while (len > 0 && isspace((unsigned char)line[len - 1])) {
    line[--len] = '\0';
  }
  size_t start = 0;
  while (line[start] != '\0' && isspace((unsigned char)line[start])) {
    start++;
  }
  if (start > 0) {
    memmove(line, line + start, strlen(line + start) + 1);
  }
}

static int str_equal_ci(const char *a, const char *b) {
#ifdef _WIN32
  return _stricmp(a, b);
#else
  return strcasecmp(a, b);
#endif
}

static int run_cli(gcs_context_t *ctx) {
  print_help();

  char line[512];
  while (1) {
    printf("gcs> ");
    fflush(stdout);
    if (fgets(line, sizeof(line), stdin) == NULL) {
      break;
    }

    trim_line(line);
    if (line[0] == '\0') {
      continue;
    }

    char *command = strtok(line, " \t");
    if (command == NULL) {
      continue;
    }

    for (char *p = command; *p != '\0'; p++) {
      *p = (char)tolower((unsigned char)*p);
    }

    if (strcmp(command, "help") == 0 || strcmp(command, "h") == 0) {
      print_help();
    } else if (strcmp(command, "quit") == 0 || strcmp(command, "q") == 0 || strcmp(command, "exit") == 0) {
      break;
    } else if (strcmp(command, "hb") == 0) {
      print_heartbeat_status(ctx);
    } else if (strcmp(command, "cancel") == 0) {
      cancel_operation(ctx);
    } else if (strcmp(command, "params") == 0 || strcmp(command, "p") == 0) {
      fetch_all_parameters(ctx);
    } else if (strcmp(command, "pr") == 0) {
      char *name = strtok(NULL, " \t");
      if (name == NULL) {
        printf("Usage: pr <name>\n");
      } else {
        read_parameter(ctx, name);
      }
    } else if (strcmp(command, "pw") == 0) {
      char *name = strtok(NULL, " \t");
      char *value = strtok(NULL, " \t");
      if (name == NULL || value == NULL) {
        printf("Usage: pw <name> <value>\n");
      } else {
        write_parameter(ctx, name, value);
      }
    } else if (strcmp(command, "mu") == 0) {
      upload_mission(ctx);
    } else if (strcmp(command, "md") == 0) {
      download_mission(ctx);
    } else if (strcmp(command, "mc") == 0) {
      clear_mission(ctx);
    } else if (strcmp(command, "ms") == 0) {
      char *seq_str = strtok(NULL, " \t");
      if (seq_str == NULL) {
        printf("Usage: ms <seq>\n");
      } else {
        set_mission_current(ctx, (int)strtol(seq_str, NULL, 10));
      }
    } else if (strcmp(command, "rm") == 0) {
      char *msg_id_str = strtok(NULL, " \t");
      if (msg_id_str == NULL) {
        printf("Usage: rm <msgId>  (e.g. rm %u for ATTITUDE)\n", attitude_MSG_ID);
      } else {
        request_message(ctx, (int)strtol(msg_id_str, NULL, 10));
      }
    } else if (strcmp(command, "si") == 0) {
      char *msg_id_str = strtok(NULL, " \t");
      char *interval_str = strtok(NULL, " \t");
      if (msg_id_str == NULL || interval_str == NULL) {
        printf("Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)\n");
      } else {
        set_message_interval(
          ctx,
          (int)strtol(msg_id_str, NULL, 10),
          (int)strtol(interval_str, NULL, 10)
        );
      }
    } else if (strcmp(command, "att") == 0) {
      char *seconds_str = strtok(NULL, " \t");
      int seconds = seconds_str != NULL ? (int)strtol(seconds_str, NULL, 10) : 5;
      stream_attitude(ctx, seconds);
    } else if (strcmp(command, "arm") == 0) {
      char *force_arg = strtok(NULL, " \t");
      int force = force_arg != NULL && str_equal_ci(force_arg, "force") == 0;
      arm_vehicle(ctx, force);
    } else if (strcmp(command, "disarm") == 0) {
      char *force_arg = strtok(NULL, " \t");
      int force = force_arg != NULL && str_equal_ci(force_arg, "force") == 0;
      disarm_vehicle(ctx, force);
    } else if (strcmp(command, "rtl") == 0) {
      return_to_launch(ctx);
    } else {
      printf("Unknown command: %s (type help)\n", command);
    }

    printf("\n");
  }
  return 0;
}

int main(int argc, char **argv) {
  int baud_rate = parse_baud_rate(argc, argv, 57600);
  char *port_name = pick_serial_port();
  if (port_name == NULL) {
    return EXIT_FAILURE;
  }

  printf("\nOpening %s @ %d baud...\n", port_name, baud_rate);

  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);

  mavlink_link_t *link = serial_mavlink_link_open(port_name, baud_rate);
  free(port_name);
  if (link == NULL) {
    fprintf(stderr, "Failed to open serial port.\n");
    return EXIT_FAILURE;
  }

  mavlink_gcs_t *gcs = mavlink_gcs_connect(
    &dialect.base,
    link,
    GCS_SYSTEM_ID,
    GCS_COMPONENT_ID,
    500,
    3000,
    MAVLINK_VERSION_V2
  );
  if (gcs == NULL) {
    fprintf(stderr, "Failed to create GCS session.\n");
    serial_mavlink_link_close(link);
    return EXIT_FAILURE;
  }

  mavlink_gcs_start(gcs);
  printf("Publishing GCS heartbeats, waiting for vehicle...\n");

  uint8_t exclude[] = { GCS_SYSTEM_ID };
  mavlink_vehicle_client_t *client = NULL;
  mavlink_wait_result_t wait = mavlink_gcs_wait_for_vehicle(gcs, exclude, 1, 60000, &client);
  if (wait != MAVLINK_WAIT_OK || client == NULL) {
    fprintf(
      stderr,
      "No vehicle heartbeat within 60 s (%s). Check port, baud (current: %d; try --baud 115200), and SITL.\n",
      wait_result_string(wait),
      baud_rate
    );
    mavlink_gcs_destroy(gcs);
    serial_mavlink_link_close(link);
    return EXIT_FAILURE;
  }

  gcs_context_t ctx = {
    .gcs = gcs,
    .vehicle = client->vehicle,
    .client = client,
    .operation_cancel = NULL,
  };

  const tracked_heartbeat_t *vehicle_state = heartbeat_monitor_state_for(gcs->heartbeat_monitor, ctx.vehicle);
  printf(
    "Vehicle online: sys=%u comp=%u\n",
    ctx.vehicle.system_id,
    ctx.vehicle.component_id
  );
  if (vehicle_state != NULL && vehicle_state->heartbeat != NULL) {
    const heartbeat_t *hb = (const heartbeat_t *)vehicle_state->heartbeat;
    printf("  type=%d autopilot=%d status=%d\n", (int)hb->type, (int)hb->autopilot, (int)hb->system_status);
  }

  printf("\n=== Phase 2: parameter sync ===\n");
  fetch_all_parameters(&ctx);

  printf("\n=== Interactive CLI ===\n");
  run_cli(&ctx);

  printf("Shutting down...\n");
  if (ctx.operation_cancel != NULL) {
    mavlink_cancellation_token_cancel(ctx.operation_cancel);
    mavlink_cancellation_token_dispose(ctx.operation_cancel);
  }
  mavlink_vehicle_client_destroy(client);
  mavlink_gcs_destroy(gcs);
  serial_mavlink_link_close(link);
  return EXIT_SUCCESS;
}
