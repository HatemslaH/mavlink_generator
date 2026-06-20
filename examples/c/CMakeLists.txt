cmake_minimum_required(VERSION 3.16)
project(mavlink_sitl_gcs C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(GENERATED_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../generated/c")

add_executable(sitl_gcs
  src/sitl_gcs.c
  src/serial_link.c
  src/port_picker.c
  src/sample_mission.c
  ${GENERATED_DIR}/protocols/mavlink_link.c
  ${GENERATED_DIR}/protocols/mavlink_cancellation.c
  ${GENERATED_DIR}/protocols/mavlink_session.c
  ${GENERATED_DIR}/protocols/param_codec.c
  ${GENERATED_DIR}/protocols/heartbeat_protocol.c
  ${GENERATED_DIR}/protocols/parameter_protocol.c
  ${GENERATED_DIR}/protocols/mission_protocol.c
  ${GENERATED_DIR}/protocols/command_protocol.c
  ${GENERATED_DIR}/protocols/mavlink_vehicle_client.c
)

target_include_directories(sitl_gcs PRIVATE
  include
  ${GENERATED_DIR}
)

if(NOT WIN32)
  target_link_libraries(sitl_gcs PRIVATE pthread)
endif()
