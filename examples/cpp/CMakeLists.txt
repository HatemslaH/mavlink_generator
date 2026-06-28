cmake_minimum_required(VERSION 3.16)
project(mavlink_sitl_gcs CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(
  GENERATED_CPP_DIR
  "${CMAKE_CURRENT_SOURCE_DIR}/../../generated/cpp"
  CACHE PATH "Generated mavlink C++ bindings (cargo run -- --lang cpp)"
)

if(NOT EXISTS "${GENERATED_CPP_DIR}/mavlink_protocols.hpp")
  message(FATAL_ERROR
    "Generated C++ bindings not found at ${GENERATED_CPP_DIR}.\n"
    "Run from the repository root:\n"
    "  cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang cpp"
  )
endif()

file(GLOB MAVLINK_PROTOCOL_SOURCES CONFIGURE_DEPENDS
  "${GENERATED_CPP_DIR}/protocols/*.cpp"
)

add_executable(sitl_gcs
  src/sitl_gcs.cpp
  src/serial_link.cpp
  src/port_picker.cpp
  ${MAVLINK_PROTOCOL_SOURCES}
)

target_include_directories(sitl_gcs PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
  "${GENERATED_CPP_DIR}"
)

if(WIN32)
  target_compile_definitions(sitl_gcs PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
endif()

if(NOT WIN32)
  find_package(Threads REQUIRED)
  target_link_libraries(sitl_gcs PRIVATE Threads::Threads)
endif()
