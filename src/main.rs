use std::path::{Path, PathBuf};

use mavlink_generator::{TargetLanguage, generate_code};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> mavlink_generator::Result<()> {
    let definitions_dir = Path::new("mavlink/message_definitions/v1.0");
    let destination_dir = Path::new("lib/mavlink/dialects");

    std::fs::create_dir_all(destination_dir)?;

    let xml_paths: Vec<PathBuf> = std::fs::read_dir(definitions_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "xml")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("rt_rc.xml"))
        })
        .collect();

    if xml_paths.is_empty() {
        return Err(mavlink_generator::GeneratorError::Format(format!(
            "No dialect XML files found in {}",
            definitions_dir.display()
        )));
    }

    for xml_path in xml_paths {
        println!("{}", xml_path.display());

        let file_stem = xml_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                mavlink_generator::GeneratorError::Format(format!(
                    "Invalid dialect file name: {}",
                    xml_path.display()
                ))
            })?;

        let dart_path = destination_dir.join(format!("{file_stem}.dart").to_lowercase());
        generate_code(&dart_path, &xml_path, TargetLanguage::Dart)?;
        println!("  -> {}", dart_path.display());
    }

    Ok(())
}
