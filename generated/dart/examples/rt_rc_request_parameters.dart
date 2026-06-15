// ignore_for_file: avoid_print

import 'common.dart';

/// Virtual parameter service for the `rt_rc` dialect.
///
/// Follows https://mavlink.io/en/services/parameter.html:
/// PARAM_REQUEST_LIST / PARAM_REQUEST_READ from GCS, PARAM_VALUE from drone.
void main() {
  final dialect = MavlinkDialectRt_rc();

  // 1. GCS requests the full onboard parameter set.
  final listRequest = ParamRequestList(
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );
  final listFrame = frameFromGcs(listRequest, sequence: 1);
  logFrame('GCS ->', listFrame);
  roundTripMessage(dialect, listRequest);

  // 2. Drone responds with PARAM_VALUE messages (simulated subset).
  final simulatedParams = <({String id, double value, int index})>[
    (id: 'SYSID_THISMAV', value: 1, index: 0),
    (id: 'SYSID_MYGCS', value: 255, index: 1),
    (id: 'COMPASS_ENABLE', value: 1, index: 2),
  ];

  for (final param in simulatedParams) {
    final value = ParamValue(
      paramValue: param.value,
      paramCount: simulatedParams.length,
      paramIndex: param.index,
      paramId: paramIdFromString(param.id),
      paramType: MavParamType.mavParamTypeReal32,
    );
    final valueFrame = frameFromDrone(value, sequence: param.index + 10);
    logFrame('Drone ->', valueFrame);
    final parsed = roundTripMessage(dialect, value);
    if (parsed is ParamValue) {
      print(
        '  PARAM_VALUE [${param.index + 1}/${simulatedParams.length}] '
        '${paramIdToString(parsed.paramId)}=${parsed.paramValue}',
      );
    }
  }

  // 3. GCS requests one parameter by name (param_index = -1).
  const paramName = 'SYSID_THISMAV';
  final readRequest = ParamRequestRead(
    paramIndex: -1,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
    paramId: paramIdFromString(paramName),
  );
  final readFrame = frameFromGcs(readRequest, sequence: 50);
  logFrame('GCS ->', readFrame);
  final parsedRead = roundTripMessage(dialect, readRequest);
  if (parsedRead is ParamRequestRead) {
    print('  PARAM_REQUEST_READ id=${paramIdToString(parsedRead.paramId)}');
  }

  // 4. Drone answers with the matching PARAM_VALUE.
  final singleValue = ParamValue(
    paramValue: 1,
    paramCount: simulatedParams.length,
    paramIndex: 0,
    paramId: paramIdFromString(paramName),
    paramType: MavParamType.mavParamTypeReal32,
  );
  final singleFrame = frameFromDrone(singleValue, sequence: 51);
  logFrame('Drone ->', singleFrame);
  roundTripMessage(dialect, singleValue);
}
