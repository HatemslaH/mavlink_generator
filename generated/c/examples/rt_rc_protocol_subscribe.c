#include <stdio.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif
#include "protocols_common.h"
static int attitude_samples;
static void on_attitude(mavlink_session_t *session, const mavlink_frame_t *frame,
  void *parsed_message, void *user_data) {
  (void)session; (void)frame; (void)user_data;
  if (parsed_message != NULL) attitude_samples++;
}
int main(void) {
  mavlink_dialect_rt_rc_t dialect;
  mavlink_dialect_rt_rc_init(&dialect);
  virtual_mavlink_link_t link = virtual_mavlink_link_create(&dialect.base);
  mavlink_message_subscription_t *subscription = mavlink_session_listen_message(
    link.gcs, attitude_MSG_ID, DRONE_SYSTEM_ID, 0, on_attitude, NULL);
  attitude_t attitude = { .time_boot_ms = 1000, .roll = 0.1f, .pitch = -0.05f, .yaw = 1.57f };
  uint8_t payload[attitude_ENCODED_LENGTH];
  attitude_serialize(&attitude, payload);
  mavlink_session_send(link.drone, attitude_MSG_ID, attitude_CRC_EXTRA, payload, attitude_ENCODED_LENGTH);
#ifdef _WIN32
  Sleep(50);
#else
  usleep(50000);
#endif
  mavlink_message_subscription_cancel(subscription);
  printf("Received %d ATTITUDE samples via listen_message\n", attitude_samples);
  virtual_mavlink_link_close(&link);
  return 0;
}
