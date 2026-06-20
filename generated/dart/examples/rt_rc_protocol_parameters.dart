// ignore_for_file: avoid_print

import 'protocols_common.dart';

/// Parameter protocol example for the `rt_rc` dialect.
///
/// Uses [ParameterProtocol] on the GCS side and [ParameterServer] on the
/// vehicle side. The link is transport-agnostic and can be swapped for USB,
/// UDP, TCP, or any custom [MavlinkLink] implementation.
Future<void> main() async {
  final dialect = MavlinkDialectRt_rc();
  final link = createVirtualLink(dialect);

  final parameterServer = ParameterServer(
    session: link.drone,
    initialValues: {
      'SYSID_THISMAV': (value: 1, type: MavParamType.mavParamTypeInt32),
      'SYSID_MYGCS': (value: 255, type: MavParamType.mavParamTypeInt32),
      'COMPASS_ENABLE': (value: 1, type: MavParamType.mavParamTypeInt32),
    },
  );

  final parameterProtocol = ParameterProtocol(
    session: link.gcs,
    targetSystem: droneSystemId,
    targetComponent: droneComponentId,
  );

  final allParams = await parameterProtocol.fetchAll();
  print('Fetched ${allParams.length} parameters:');
  for (final param in allParams) {
    print('  ${param.id}=${param.value} (${param.type})');
  }

  final single = await parameterProtocol.readByName('SYSID_THISMAV');
  print('Read SYSID_THISMAV=${single.value}');

  final updated = await parameterProtocol.write(
    name: 'COMPASS_ENABLE',
    value: 0,
    type: MavParamType.mavParamTypeInt32,
  );
  print('Wrote COMPASS_ENABLE=${updated.value}');

  await parameterServer.close();
  await closeVirtualLink(
    bus: link.bus,
    gcs: link.gcs,
    drone: link.drone,
  );
}
