#include <cstdio>
#include <map>

#include "protocols_common.hpp"

/// Parameter protocol example for the `rt_rc` dialect.
int main() {
  mavlink::mavlink_dialect_rt_rc_t dialect;
  mavlink::mavlink_dialect_rt_rc_init(dialect);

  const std::map<std::string, mavlink::ParamStoredValue> initial_values = {
    {"SYSID_THISMAV", {1.0, mavlink::MAV_PARAM_TYPE_INT32}},
    {"SYSID_MYGCS", {255.0, mavlink::MAV_PARAM_TYPE_INT32}},
    {"COMPASS_ENABLE", {1.0, mavlink::MAV_PARAM_TYPE_INT32}},
  };

  auto link = mavlink::create_virtual_link(dialect);
  mavlink::ParameterServer parameter_server(link.drone.get(), &initial_values);
  mavlink::ParameterProtocol parameter_protocol(
    link.gcs.get(),
    mavlink::drone_system_id,
    mavlink::drone_component_id
  );

  const auto all_params = parameter_protocol.fetch_all(
    [](const mavlink::ParamEntry& entry, int received, int expected) {
      std::printf("  [%d/%d] %s=%f\n", received, expected, entry.id.c_str(), entry.value);
    }
  );
  std::printf(
    "Fetched %zu parameters (cache size=%zu)\n",
    all_params.size(),
    parameter_protocol.cache().size()
  );

  const auto single = parameter_protocol.read_by_name("SYSID_THISMAV");
  std::printf("Read SYSID_THISMAV=%f\n", single.value);

  const auto updated = parameter_protocol.write_by_name("COMPASS_ENABLE", 0);
  std::printf("Wrote COMPASS_ENABLE=%f (%d)\n", updated.value, static_cast<int>(updated.type));

  parameter_server.close();
  mavlink::close_virtual_link(link);
  return 0;
}
