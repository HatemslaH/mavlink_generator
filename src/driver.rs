use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    DialectDocument, GeneratorError, Result, TargetLanguage, dialects_relative_dir, generate_code,
    generate_example_files, generate_runtime_files,
};

pub const DEFAULT_DEFINITIONS_DIR: &str = "mavlink/message_definitions/v1.0";
pub const DEFAULT_OUTPUT_ROOT: &str = "generated";
pub const DEFAULT_DIALECT_FILTER: &str = "rt_rc";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    #[serde(default)]
    pub languages: Vec<String>,
    pub dialect: Option<String>,
    #[serde(default)]
    pub all_dialects: bool,
    pub definitions_dir: PathBuf,
    #[serde(default = "default_true")]
    pub runtime: bool,
    #[serde(default = "default_true")]
    pub examples: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            output: PathBuf::from(DEFAULT_OUTPUT_ROOT),
            languages: Vec::new(),
            dialect: None,
            all_dialects: false,
            definitions_dir: PathBuf::from(DEFAULT_DEFINITIONS_DIR),
            runtime: true,
            examples: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub id: String,
    pub display_name: String,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResult {
    pub stem: String,
    pub version: i32,
    pub enum_count: usize,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateProgress {
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub message: String,
}

pub fn list_languages() -> Vec<LanguageInfo> {
    all_languages()
        .into_iter()
        .map(|language| LanguageInfo {
            id: language_id(language).to_string(),
            display_name: language.display_name().to_string(),
            output_dir: language.output_dir_name().to_string(),
        })
        .collect()
}

pub fn resolve_inputs(options: &GenerateOptions) -> Result<Vec<PathBuf>> {
    if options.inputs.is_empty() {
        return collect_xml_from_dir(&options.definitions_dir, dialect_filter(options));
    }

    let mut xml_paths = Vec::new();
    for input in &options.inputs {
        if input.is_dir() {
            xml_paths.extend(collect_xml_from_dir(input, dialect_filter(options))?);
        } else {
            xml_paths.push(input.clone());
        }
    }

    if xml_paths.is_empty() {
        return Err(GeneratorError::Format(
            "No dialect XML files matched the given inputs".into(),
        ));
    }

    xml_paths.sort();
    xml_paths.dedup();
    Ok(xml_paths)
}

pub fn validate_dialects(paths: &[PathBuf]) -> Result<Vec<ValidateResult>> {
    paths
        .iter()
        .map(|path| {
            let document = DialectDocument::parse(path)?;
            let stem = dialect_stem(path)?;
            Ok(ValidateResult {
                stem,
                version: document.version,
                enum_count: document.enums.enums().len(),
                message_count: document.messages.messages().len(),
            })
        })
        .collect()
}

pub fn run_generate<F>(options: &GenerateOptions, mut on_progress: F) -> Result<()>
where
    F: FnMut(GenerateProgress),
{
    let languages = selected_languages(&options.languages);
    let xml_paths = resolve_inputs(options)?;

    for language in languages {
        let language_dir = options.output.join(language.output_dir_name());
        let dialects_dir = language_dir.join(dialects_relative_dir(language));
        std::fs::create_dir_all(&dialects_dir)?;

        let mut stems_for_language = Vec::with_capacity(xml_paths.len());

        for xml_path in &xml_paths {
            let stem = dialect_stem(xml_path)?;
            stems_for_language.push(stem.clone());

            on_progress(GenerateProgress {
                stage: "dialect".into(),
                language: Some(language.display_name().to_string()),
                path: Some(xml_path.display().to_string()),
                message: format!("[{}] {}", language.display_name(), xml_path.display()),
            });

            let dst = dialects_dir.join(format!("{stem}.{}", language.file_extension()));
            generate_code(&dst, xml_path, language)?;

            on_progress(GenerateProgress {
                stage: "dialect".into(),
                language: Some(language.display_name().to_string()),
                path: Some(dst.display().to_string()),
                message: format!("  -> {}", dst.display()),
            });
        }

        prune_stale_dialect_files(&dialects_dir, language, &stems_for_language)?;

        if options.runtime {
            generate_runtime_files(&language_dir, language, &stems_for_language)?;
            on_progress(GenerateProgress {
                stage: "runtime".into(),
                language: Some(language.display_name().to_string()),
                path: Some(language_dir.display().to_string()),
                message: format!(
                    "Generated {} runtime files in {}",
                    language.display_name(),
                    language_dir.display()
                ),
            });
        }

        if options.examples {
            generate_example_files(&language_dir, language, &stems_for_language)?;
            on_progress(GenerateProgress {
                stage: "examples".into(),
                language: Some(language.display_name().to_string()),
                path: Some(language_dir.join("examples").display().to_string()),
                message: format!(
                    "Generated {} examples in {}",
                    language.display_name(),
                    language_dir.join("examples").display()
                ),
            });
        }
    }

    Ok(())
}

pub fn language_id(language: TargetLanguage) -> &'static str {
    language.output_dir_name()
}

pub fn parse_language_id(id: &str) -> Option<TargetLanguage> {
    all_languages()
        .into_iter()
        .find(|language| language_id(*language).eq_ignore_ascii_case(id))
}

fn selected_languages(language_ids: &[String]) -> Vec<TargetLanguage> {
    if language_ids.is_empty() {
        return all_languages();
    }

    language_ids
        .iter()
        .filter_map(|id| parse_language_id(id))
        .collect()
}

fn all_languages() -> Vec<TargetLanguage> {
    vec![
        TargetLanguage::Dart,
        TargetLanguage::C,
        TargetLanguage::Cpp,
        TargetLanguage::Python,
        TargetLanguage::JavaScript,
        TargetLanguage::TypeScript,
        TargetLanguage::CSharp,
        TargetLanguage::Rust,
    ]
}

fn dialect_filter(options: &GenerateOptions) -> Option<&str> {
    if options.all_dialects {
        return None;
    }

    Some(options.dialect.as_deref().unwrap_or(DEFAULT_DIALECT_FILTER))
}

fn collect_xml_from_dir(dir: &Path, dialect_stem_filter: Option<&str>) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(GeneratorError::Format(format!(
            "Not a directory: {}",
            dir.display()
        )));
    }

    let mut xml_paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "xml"))
        .filter(|path| {
            dialect_stem_filter.is_none_or(|stem| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(stem))
            })
        })
        .collect();

    if xml_paths.is_empty() {
        let filter_note = dialect_stem_filter
            .map(|stem| format!(" matching dialect '{stem}'"))
            .unwrap_or_default();
        return Err(GeneratorError::Format(format!(
            "No dialect XML files found in {}{filter_note}",
            dir.display()
        )));
    }

    xml_paths.sort();
    Ok(xml_paths)
}

fn prune_stale_dialect_files(
    dialects_dir: &Path,
    language: TargetLanguage,
    active_stems: &[String],
) -> Result<()> {
    if !dialects_dir.is_dir() {
        return Ok(());
    }

    let extension = language.file_extension();
    let active: HashSet<&str> = active_stems.iter().map(String::as_str).collect();

    for entry in fs::read_dir(dialects_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(extension) {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if !active.contains(stem) {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn dialect_stem(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_lowercase)
        .ok_or_else(|| {
            GeneratorError::Format(format!("Invalid dialect file name: {}", path.display()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_resolve_rt_rc() {
        let options = GenerateOptions::default();

        let xml_paths = resolve_inputs(&options).expect("default inputs should resolve");
        assert_eq!(xml_paths.len(), 1);
        assert!(xml_paths[0].ends_with("rt_rc.xml"));
    }

    #[test]
    fn selected_languages_defaults_to_all() {
        assert_eq!(selected_languages(&[]).len(), 8);
    }

    #[test]
    fn parse_language_id_accepts_output_dir_names() {
        assert_eq!(parse_language_id("rust"), Some(TargetLanguage::Rust));
        assert_eq!(parse_language_id("csharp"), Some(TargetLanguage::CSharp));
    }
}
