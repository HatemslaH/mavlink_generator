use std::path::Path;

use mavlink_generator::{
    TargetLanguage, dialects_output_dir, generate_dialect, generate_example_files,
    generate_runtime_files, language_output_dir,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> mavlink_generator::Result<()> {
    let definitions_dir = Path::new("mavlink/message_definitions/v1.0");
    let languages = [
        TargetLanguage::Dart,
        TargetLanguage::C,
        TargetLanguage::Python,
    ];

    let xml_paths: Vec<_> = std::fs::read_dir(definitions_dir)?
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

    for language in languages {
        let output_dir = language_output_dir(language);
        let dialects_dir = dialects_output_dir(language);
        std::fs::create_dir_all(&dialects_dir)?;

        let mut dialect_stems = Vec::with_capacity(xml_paths.len());

        for xml_path in &xml_paths {
            println!("[{}] {}", language.display_name(), xml_path.display());

            let file_stem = xml_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    mavlink_generator::GeneratorError::Format(format!(
                        "Invalid dialect file name: {}",
                        xml_path.display()
                    ))
                })?
                .to_lowercase();

            generate_dialect(xml_path, language, &file_stem)?;
            println!(
                "  -> {}",
                dialects_dir
                    .join(format!("{file_stem}.{}", language.file_extension()))
                    .display()
            );
            dialect_stems.push(file_stem);
        }

        generate_runtime_files(&output_dir, language, &dialect_stems)?;
        println!(
            "Generated {} runtime files in {}",
            language.display_name(),
            output_dir.display()
        );

        generate_example_files(&output_dir, language, &dialect_stems)?;
        println!(
            "Generated {} examples in {}",
            language.display_name(),
            mavlink_generator::examples_output_dir(language).display()
        );
    }

    Ok(())
}
