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
        std::path::PathBuf::from("generated/dart/dialects/rt_rc.dart")
    );

    let py_path = dialect_output_path(TargetLanguage::Python, "common");
    assert_eq!(
        py_path,
        std::path::PathBuf::from("generated/py/dialects/common.py")
    );
}

#[test]
fn generates_dart_runtime_files() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_dart_runtime_test");
    let dialect_stems = vec!["rt_rc".to_string(), "common".to_string()];

    generate_runtime_files(&output_dir, TargetLanguage::Dart, &dialect_stems)
        .expect("runtime generation should succeed");

    let entry_point = output_dir.join("mavlink.dart");
    let content = std::fs::read_to_string(&entry_point).expect("mavlink.dart should exist");
    assert!(content.contains("export 'dialects/rt_rc.dart';"));
    assert!(content.contains("export 'dialects/common.dart';"));
    assert!(content.contains("export 'mavlink_parser.dart';"));

    assert!(output_dir.join("crc.dart").is_file());
    assert!(output_dir.join("mavlink_parser.dart").is_file());

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

    assert_eq!(
        examples_output_dir(TargetLanguage::Dart),
        std::path::PathBuf::from("generated/dart/examples")
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn unimplemented_example_languages_return_error() {
    let output_dir = std::env::temp_dir().join("mavlink_generator_examples_stub_test");
    let dialect_stems = vec!["rt_rc".to_string()];

    let python_err = generate_example_files(&output_dir, TargetLanguage::Python, &dialect_stems)
        .expect_err("python examples should not be implemented yet");
    assert!(python_err.to_string().contains("Python"));

    let c_err = generate_example_files(&output_dir, TargetLanguage::C, &dialect_stems)
        .expect_err("c examples should not be implemented yet");
    assert!(c_err.to_string().contains("Example generation"));
}

#[test]
fn unimplemented_languages_return_error() {
    let output = std::env::temp_dir().join("rt_rc_generated.py");
    let xml = "mavlink/message_definitions/v1.0/rt_rc.xml";

    let python_err =
        mavlink_generator::generate_code(&output, xml, mavlink_generator::TargetLanguage::Python)
            .expect_err("python generation should not be implemented yet");
    assert!(python_err.to_string().contains("Python"));

    let c_err =
        mavlink_generator::generate_code(&output, xml, mavlink_generator::TargetLanguage::C)
            .expect_err("c generation should not be implemented yet");
    assert!(c_err.to_string().contains("C code generation"));
}
