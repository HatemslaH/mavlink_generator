use mavlink_generator::{
    DialectDocument, TargetLanguage, dialect_output_path, examples_output_dir,
    generate_example_files, generate_runtime_files,
};

#[test]
fn parses_rt_rc_dialect() {
    let doc = DialectDocument::parse("mavlink/message_definitions/v1.0/rt_rc.xml")
        .expect("rt_rc dialect should parse");

    assert_eq!(doc.version, 3);
    assert!(!doc.enums.enums().is_empty());
    assert!(!doc.messages.messages().is_empty());

    let rt_rc_channels = doc
        .messages
        .messages()
        .iter()
        .find(|message| message.name == "RT_RC_CHANNELS")
        .expect("RT_RC_CHANNELS should be present");

    assert_eq!(rt_rc_channels.id, 45000);
    assert_eq!(rt_rc_channels.calculate_crc_extra().unwrap(), 247);
}

#[test]
fn generates_rt_rc_dart_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.dart");
    mavlink_generator::generate_dart_code(&output, "mavlink/message_definitions/v1.0/rt_rc.xml")
        .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("class MavlinkDialectRt_rc implements MavlinkDialect"));
    assert!(content.contains("static const int crcExtra = 247;"));
    assert!(content.contains("enum RtRcControlId"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn dialect_output_path_uses_generated_layout() {
    let path = dialect_output_path(TargetLanguage::Dart, "rt_rc");
    assert_eq!(
        path,
        std::path::PathBuf::from("generated/dart/lib/dialects/rt_rc.dart")
    );

    let py_path = dialect_output_path(TargetLanguage::Python, "common");
    assert_eq!(
        py_path,
        std::path::PathBuf::from("generated/py/dialects/common.py")
    );

    let cpp_path = dialect_output_path(TargetLanguage::Cpp, "rt_rc");
    assert_eq!(
        cpp_path,
        std::path::PathBuf::from("generated/cpp/dialects/rt_rc.hpp")
    );
}

#[test]
fn generates_dart_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_dart_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::Dart, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("lib/mavlink.dart");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.dart should exist");
    assert!(content.contains("export 'dialects/rt_rc.dart';"));
    assert!(content.contains("export 'dialects/common.dart';"));
    assert!(content.contains("export 'mavlink_parser.dart';"));

    assert!(output_dir.join("lib/crc.dart").is_file());
    assert!(output_dir.join("pubspec.yaml").is_file());
    assert!(output_dir.join("lib/mavlink.dart").is_file());
    assert!(output_dir.join("lib/mavlink_protocols.dart").is_file());
    assert!(output_dir.join("lib/mavlink_parser.dart").is_file());
    assert!(
        output_dir
            .join("lib/protocols/mission_protocol.dart")
            .is_file()
    );
    assert!(
        output_dir
            .join("lib/protocols/parameter_protocol.dart")
            .is_file()
    );
    assert!(
        output_dir
            .join("lib/protocols/command_protocol.dart")
            .is_file()
    );
    assert!(
        output_dir
            .join("lib/protocols/heartbeat_protocol.dart")
            .is_file()
    );
    assert!(
        output_dir
            .join("lib/protocols/mavlink_cancellation.dart")
            .is_file()
    );
    assert!(
        output_dir
            .join("lib/protocols/mavlink_vehicle_client.dart")
            .is_file()
    );

    let session_source =
        std::fs::read_to_string(output_dir.join("lib/protocols/mavlink_session.dart"))
            .expect("mavlink_session.dart should exist");
    assert!(session_source.contains("listenMessage"));
    assert!(session_source.contains("onMessage"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("lib/protocols/parameter_protocol.dart"))
            .expect("parameter_protocol.dart should exist");
    assert!(parameter_source.contains("fetchAllStream"));
    assert!(parameter_source.contains("writeByName"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_dart_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_dart_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::Dart, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.dart").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.dart"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRt_rc"));
    assert!(heartbeat.contains("roundTripMessage(dialect, heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.dart"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.dart"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("mavCmdSetMessageInterval"));
    assert!(telemetry.contains("mavCmdRequestMessage"));
    assert!(telemetry.contains("Attitude.msgId"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.dart"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.dart").is_file());

    let protocol_mission =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.dart"))
            .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("VirtualMavlinkBus"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.dart"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.dart"))
            .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.dart"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("waitForVehicle"));

    let protocol_vehicle =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.dart"))
            .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.dart"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listenMessage"));

    assert_eq!(
        examples_output_dir(TargetLanguage::Dart),
        std::path::PathBuf::from("generated/dart/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_c_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.h");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::C,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("typedef struct mavlink_dialect_rt_rc_t"));
    assert!(content.contains("#define heartbeat_CRC_EXTRA"));
    assert!(content.contains("typedef enum"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_c_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_c_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::C, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.h");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.h should exist");
    assert!(content.contains("#include \"dialects/rt_rc.h\""));
    assert!(content.contains("#include \"mavlink_frame.h\""));

    assert!(output_dir.join("crc.h").is_file());
    assert!(output_dir.join("mavlink_parser.h").is_file());
    assert!(output_dir.join("mavlink_protocols.h").is_file());
    assert!(output_dir.join("protocols/mission_protocol.h").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.h").is_file());
    assert!(output_dir.join("protocols/command_protocol.h").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.h").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.h")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.h")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.h"))
        .expect("mavlink_session.h should exist");
    assert!(session_source.contains("mavlink_session_listen_message"));
    assert!(session_source.contains("mavlink_session_wait_for_message"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.h"))
            .expect("parameter_protocol.h should exist");
    assert!(parameter_source.contains("parameter_protocol_fetch_all"));
    assert!(parameter_source.contains("parameter_protocol_write_by_name"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_c_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_c_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::C, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.h").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.c"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("mavlink_dialect_rt_rc_init"));
    assert!(heartbeat.contains("dialect.base.parse"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.c"))
        .expect("mission example should exist");
    assert!(mission.contains("mission_count_t"));
    assert!(mission.contains("mission_request_t"));
    assert!(mission.contains("mission_ack_t"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.c"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("attitude_MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.c"))
        .expect("parameters example should exist");
    assert!(params.contains("param_request_list_t"));
    assert!(params.contains("param_request_read_t"));
    assert!(params.contains("param_value_t"));

    assert!(examples_dir.join("protocols_common.h").is_file());
    assert!(examples_dir.join("protocols_common.c").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.c"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("mission_protocol_create"));
    assert!(protocol_mission.contains("mission_server_create"));
    assert!(protocol_mission.contains("virtual_mavlink_link_create"));

    let protocol_params = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.c"))
        .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("parameter_protocol_create"));
    assert!(protocol_params.contains("parameter_server_create"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.c"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("command_protocol_create"));
    assert!(protocol_command.contains("command_server_create"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.c"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("heartbeat_monitor_create"));
    assert!(protocol_heartbeat.contains("heartbeat_publisher_create"));
    assert!(protocol_heartbeat.contains("heartbeat_monitor_wait_for_vehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.c"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("mavlink_gcs_connect"));
    assert!(protocol_vehicle.contains("mavlink_vehicle_client"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.c"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("mavlink_session_listen_message"));

    assert_eq!(
        examples_output_dir(TargetLanguage::C),
        std::path::PathBuf::from("generated/c/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_cpp_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.hpp");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::Cpp,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("struct mavlink_dialect_rt_rc_t"));
    assert!(content.contains("heartbeat_CRC_EXTRA"));
    assert!(content.contains("enum MAV_TYPE"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_cpp_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_cpp_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::Cpp, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.hpp");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.hpp should exist");
    assert!(content.contains("#include \"dialects/rt_rc.hpp\""));
    assert!(content.contains("#include \"dialects/common.hpp\""));
    assert!(content.contains("#include \"mavlink_frame.hpp\""));

    assert!(output_dir.join("crc.hpp").is_file());
    assert!(output_dir.join("mavlink_parser.hpp").is_file());
    assert!(output_dir.join("mavlink_protocols.hpp").is_file());
    assert!(output_dir.join("protocols/mission_protocol.hpp").is_file());
    assert!(
        output_dir
            .join("protocols/parameter_protocol.hpp")
            .is_file()
    );
    assert!(output_dir.join("protocols/command_protocol.hpp").is_file());
    assert!(
        output_dir
            .join("protocols/heartbeat_protocol.hpp")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.hpp")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.hpp")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.hpp"))
        .expect("mavlink_session.hpp should exist");
    assert!(session_source.contains("listen_message"));
    assert!(session_source.contains("wait_for_message"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.hpp"))
            .expect("parameter_protocol.hpp should exist");
    assert!(parameter_source.contains("fetch_all"));
    assert!(parameter_source.contains("write_by_name"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_cpp_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_cpp_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::Cpp, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.hpp").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.cpp"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("mavlink_dialect_rt_rc_init"));
    assert!(heartbeat.contains("dialect.base.parse"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.cpp"))
        .expect("mission example should exist");
    assert!(mission.contains("mission_item_t"));
    assert!(mission.contains("mission_request_t"));
    assert!(mission.contains("mission_ack_t"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.cpp"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("attitude_MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.cpp"))
        .expect("parameters example should exist");
    assert!(params.contains("param_request_list_t"));
    assert!(params.contains("param_request_read_t"));
    assert!(params.contains("param_value_t"));

    assert!(examples_dir.join("protocols_common.hpp").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.cpp"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("create_virtual_link"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.cpp"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.cpp"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.cpp"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("wait_for_vehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.cpp"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.cpp"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listen_message"));

    assert_eq!(
        examples_output_dir(TargetLanguage::Cpp),
        std::path::PathBuf::from("generated/cpp/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_python_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.py");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::Python,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("class MavlinkDialectRt_rc(MavlinkDialect)"));
    assert!(content.contains("CRC_EXTRA: ClassVar[int] = 247"));
    assert!(content.contains("class RtRcControlId(IntEnum)"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_python_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_python_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::Python, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.py");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.py should exist");
    assert!(content.contains("from dialects.rt_rc import *"));
    assert!(content.contains("from dialects.common import *"));
    assert!(content.contains("from mavlink_parser import MavlinkParser"));

    assert!(output_dir.join("crc.py").is_file());
    assert!(output_dir.join("mavlink_parser.py").is_file());
    assert!(output_dir.join("mavlink_protocols.py").is_file());
    assert!(output_dir.join("dialects/__init__.py").is_file());
    assert!(output_dir.join("protocols/mission_protocol.py").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.py").is_file());
    assert!(output_dir.join("protocols/command_protocol.py").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.py").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.py")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.py")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.py"))
        .expect("mavlink_session.py should exist");
    assert!(session_source.contains("listen_message"));
    assert!(session_source.contains("on_message"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.py"))
            .expect("parameter_protocol.py should exist");
    assert!(parameter_source.contains("fetch_all_stream"));
    assert!(parameter_source.contains("write_by_name"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_python_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_python_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::Python, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.py").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.py"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRt_rc"));
    assert!(heartbeat.contains("round_trip_message(dialect, heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.py"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.py"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("Attitude.MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.py"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.py").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.py"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("VirtualMavlinkBus"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.py"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.py"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.py"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("wait_for_vehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.py"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.py"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listen_message"));

    assert_eq!(
        examples_output_dir(TargetLanguage::Python),
        std::path::PathBuf::from("generated/py/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_javascript_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.js");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::JavaScript,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("export class MavlinkDialectRt_rc extends MavlinkDialect"));
    assert!(content.contains("static CRC_EXTRA = 247"));
    assert!(content.contains("export const RtRcControlId"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_javascript_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_javascript_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::JavaScript, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.js");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.js should exist");
    assert!(content.contains("export * from './dialects/rt_rc.js';"));
    assert!(content.contains("export * from './dialects/common.js';"));
    assert!(content.contains("export { MavlinkParser }"));

    assert!(output_dir.join("crc.js").is_file());
    assert!(output_dir.join("mavlink_parser.js").is_file());
    assert!(output_dir.join("package.json").is_file());
    assert!(output_dir.join("mavlink_protocols.js").is_file());
    assert!(output_dir.join("protocols/mission_protocol.js").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.js").is_file());
    assert!(output_dir.join("protocols/command_protocol.js").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.js").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.js")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.js")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.js"))
        .expect("mavlink_session.js should exist");
    assert!(session_source.contains("listenMessage"));
    assert!(session_source.contains("onMessage"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.js"))
            .expect("parameter_protocol.js should exist");
    assert!(parameter_source.contains("fetchAllStream"));
    assert!(parameter_source.contains("writeByName"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_javascript_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_javascript_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::JavaScript, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.js").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.js"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRt_rc"));
    assert!(heartbeat.contains("roundTripMessage(dialect, heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.js"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.js"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("Attitude.MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.js"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.js").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.js"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("VirtualMavlinkBus"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.js"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.js"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.js"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("waitForVehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.js"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.js"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listenMessage"));

    assert_eq!(
        examples_output_dir(TargetLanguage::JavaScript),
        std::path::PathBuf::from("generated/js/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_typescript_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.ts");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::TypeScript,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("export class MavlinkDialectRt_rc implements MavlinkDialect"));
    assert!(content.contains("static readonly CRC_EXTRA = 247"));
    assert!(content.contains("export enum RtRcControlId"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_typescript_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_typescript_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::TypeScript, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.ts");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.ts should exist");
    assert!(content.contains("export * from './dialects/rt_rc';"));
    assert!(content.contains("export * from './dialects/common';"));
    assert!(content.contains("export { MavlinkParser }"));

    assert!(output_dir.join("crc.ts").is_file());
    assert!(output_dir.join("mavlink_parser.ts").is_file());
    assert!(output_dir.join("mavlink_protocols.ts").is_file());
    assert!(output_dir.join("protocols/mission_protocol.ts").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.ts").is_file());
    assert!(output_dir.join("protocols/command_protocol.ts").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.ts").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.ts")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.ts")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.ts"))
        .expect("mavlink_session.ts should exist");
    assert!(session_source.contains("listenMessage"));
    assert!(session_source.contains("onMessage"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.ts"))
            .expect("parameter_protocol.ts should exist");
    assert!(parameter_source.contains("fetchAllStream"));
    assert!(parameter_source.contains("writeByName"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_typescript_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_typescript_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::TypeScript, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.ts").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.ts"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRt_rc"));
    assert!(heartbeat.contains("roundTripMessage(dialect, heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.ts"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.ts"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("Attitude.MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.ts"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.ts").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.ts"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("VirtualMavlinkBus"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.ts"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.ts"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.ts"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("waitForVehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.ts"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.ts"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listenMessage"));

    assert_eq!(
        examples_output_dir(TargetLanguage::TypeScript),
        std::path::PathBuf::from("generated/ts/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_csharp_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.cs");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::CSharp,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("public sealed class MavlinkDialectRt_rc : MavlinkDialect"));
    assert!(content.contains("public const int CrcExtra = 247"));
    assert!(content.contains("public enum RtRcControlId"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_csharp_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_csharp_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::CSharp, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.cs");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.cs should exist");
    assert!(content.contains("dialects/rt_rc.cs"));
    assert!(content.contains("MavlinkBindings"));

    assert!(output_dir.join("crc.cs").is_file());
    assert!(output_dir.join("mavlink_parser.cs").is_file());
    assert!(output_dir.join("mavlink_protocols.cs").is_file());
    assert!(output_dir.join("Mavlink.csproj").is_file());
    let csproj = std::fs::read_to_string(output_dir.join("Mavlink.csproj"))
        .expect("Mavlink.csproj should exist");
    assert!(csproj.contains(r#"<Compile Include="dialects/rt_rc.cs" />"#));
    assert!(csproj.contains(r#"<Compile Remove="dialects/**" />"#));
    assert!(output_dir.join("protocols/mission_protocol.cs").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.cs").is_file());
    assert!(output_dir.join("protocols/command_protocol.cs").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.cs").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.cs")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.cs")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.cs"))
        .expect("mavlink_session.cs should exist");
    assert!(session_source.contains("ListenMessage"));
    assert!(session_source.contains("OnMessage"));
    assert!(session_source.contains("RemovePendingWait"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.cs"))
            .expect("parameter_protocol.cs should exist");
    assert!(parameter_source.contains("FetchAllStreamAsync"));
    assert!(parameter_source.contains("WriteByNameAsync"));
    assert!(parameter_source.contains("isRetrying"));

    let vehicle_client_source =
        std::fs::read_to_string(output_dir.join("protocols/mavlink_vehicle_client.cs"))
            .expect("mavlink_vehicle_client.cs should exist");
    assert!(vehicle_client_source.contains("FromSeconds(30)"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_csharp_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_csharp_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::CSharp, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.cs").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.cs"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRt_rc"));
    assert!(heartbeat.contains("RoundTripMessage(dialect, heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.cs"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.cs"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("Attitude.MsgId"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.cs"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.cs").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.cs"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("VirtualMavlinkBus"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.cs"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.cs"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.cs"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("WaitForVehicleAsync"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.cs"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.cs"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("ListenMessage"));

    assert!(examples_dir.join("rt_rc_heartbeat.csproj").is_file());
    assert!(
        examples_dir
            .join("rt_rc_request_telemetry.csproj")
            .is_file()
    );
    assert!(examples_dir.join("rt_rc_protocol_mission.csproj").is_file());

    assert_eq!(
        examples_output_dir(TargetLanguage::CSharp),
        std::path::PathBuf::from("generated/csharp/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rt_rc_rust_file() {
    let output = std::env::temp_dir().join("rt_rc_generated.rs");
    mavlink_generator::generate_code(
        &output,
        "mavlink/message_definitions/v1.0/rt_rc.xml",
        TargetLanguage::Rust,
    )
    .expect("generation should succeed");

    let content = std::fs::read_to_string(&output).expect("generated file should exist");
    assert!(content.contains("pub struct MavlinkDialectRtRc;"));
    assert!(content.contains("pub const CRC_EXTRA: u8 = 247"));
    assert!(content.contains("pub enum RtRcControlId"));

    let _ = std::fs::remove_file(&output);
}

#[test]
fn generates_rust_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_rust_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::Rust, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("lib.rs");
    let content = std::fs::read_to_string(&entry_point).expect("lib.rs should exist");
    assert!(content.contains("pub use dialects::rt_rc::*;"));
    assert!(content.contains("pub use mavlink_parser::MavlinkParser;"));

    assert!(output_dir.join("crc.rs").is_file());
    assert!(output_dir.join("mavlink_parser.rs").is_file());
    assert!(output_dir.join("mavlink_protocols.rs").is_file());
    assert!(output_dir.join("protocols/mission_protocol.rs").is_file());
    assert!(output_dir.join("protocols/parameter_protocol.rs").is_file());
    assert!(output_dir.join("protocols/command_protocol.rs").is_file());
    assert!(output_dir.join("protocols/heartbeat_protocol.rs").is_file());
    assert!(
        output_dir
            .join("protocols/mavlink_cancellation.rs")
            .is_file()
    );
    assert!(
        output_dir
            .join("protocols/mavlink_vehicle_client.rs")
            .is_file()
    );

    let session_source = std::fs::read_to_string(output_dir.join("protocols/mavlink_session.rs"))
        .expect("mavlink_session.rs should exist");
    assert!(session_source.contains("listen_message"));
    assert!(session_source.contains("on_message"));

    let parameter_source =
        std::fs::read_to_string(output_dir.join("protocols/parameter_protocol.rs"))
            .expect("parameter_protocol.rs should exist");
    assert!(parameter_source.contains("fetch_all_stream"));
    assert!(parameter_source.contains("write_by_name"));

    let cargo =
        std::fs::read_to_string(output_dir.join("Cargo.toml")).expect("Cargo.toml should exist");
    assert!(cargo.contains("tokio"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn generates_rust_example_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_rust_examples_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    generate_example_files(&output_dir, TargetLanguage::Rust, &dialect_stems)
        .expect("example generation should succeed");

    let examples_dir = output_dir.join("examples");
    assert!(examples_dir.join("common.rs").is_file());
    assert!(examples_dir.join("README.md").is_file());

    let heartbeat = std::fs::read_to_string(examples_dir.join("rt_rc_heartbeat.rs"))
        .expect("heartbeat example should exist");
    assert!(heartbeat.contains("MavlinkDialectRtRc"));
    assert!(heartbeat.contains("round_trip_message(&dialect, &heartbeat)"));

    let mission = std::fs::read_to_string(examples_dir.join("rt_rc_mission_upload.rs"))
        .expect("mission example should exist");
    assert!(mission.contains("MissionCount"));
    assert!(mission.contains("MissionRequest"));
    assert!(mission.contains("MissionAck"));

    let telemetry = std::fs::read_to_string(examples_dir.join("rt_rc_request_telemetry.rs"))
        .expect("telemetry example should exist");
    assert!(telemetry.contains("MAV_CMD_SET_MESSAGE_INTERVAL"));
    assert!(telemetry.contains("MAV_CMD_REQUEST_MESSAGE"));
    assert!(telemetry.contains("Attitude::MSG_ID"));

    let params = std::fs::read_to_string(examples_dir.join("rt_rc_request_parameters.rs"))
        .expect("parameters example should exist");
    assert!(params.contains("ParamRequestList"));
    assert!(params.contains("ParamRequestRead"));
    assert!(params.contains("ParamValue"));

    assert!(examples_dir.join("protocols_common.rs").is_file());

    let protocol_mission = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_mission.rs"))
        .expect("protocol mission example should exist");
    assert!(protocol_mission.contains("MissionProtocol"));
    assert!(protocol_mission.contains("MissionServer"));
    assert!(protocol_mission.contains("create_virtual_link"));

    let protocol_params =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_parameters.rs"))
            .expect("protocol parameters example should exist");
    assert!(protocol_params.contains("ParameterProtocol"));
    assert!(protocol_params.contains("ParameterServer"));

    let protocol_command = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_command.rs"))
        .expect("protocol command example should exist");
    assert!(protocol_command.contains("CommandProtocol"));
    assert!(protocol_command.contains("CommandServer"));

    let protocol_heartbeat =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_heartbeat.rs"))
            .expect("protocol heartbeat example should exist");
    assert!(protocol_heartbeat.contains("HeartbeatMonitor"));
    assert!(protocol_heartbeat.contains("HeartbeatPublisher"));
    assert!(protocol_heartbeat.contains("wait_for_vehicle"));

    let protocol_vehicle = std::fs::read_to_string(examples_dir.join("rt_rc_protocol_vehicle.rs"))
        .expect("protocol vehicle example should exist");
    assert!(protocol_vehicle.contains("MavlinkGcs"));
    assert!(protocol_vehicle.contains("MavlinkVehicleClient"));

    let protocol_subscribe =
        std::fs::read_to_string(examples_dir.join("rt_rc_protocol_subscribe.rs"))
            .expect("protocol subscribe example should exist");
    assert!(protocol_subscribe.contains("listen_message"));

    assert_eq!(
        examples_output_dir(TargetLanguage::Rust),
        std::path::PathBuf::from("generated/rust/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}
