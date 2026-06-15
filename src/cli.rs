use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use mavlink_generator::{
    DialectDocument, TargetLanguage, generate_code, generate_example_files, generate_runtime_files,
};

const DEFAULT_DEFINITIONS_DIR: &str = "mavlink/message_definitions/v1.0";
const DEFAULT_OUTPUT_ROOT: &str = "generated";
const DEFAULT_DIALECT_FILTER: &str = "rt_rc";

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LanguageArg {
    Dart,
    Python,
    C,
    Cpp,
    JavaScript,
    TypeScript,
    CSharp,
    Rust,
}

impl From<LanguageArg> for TargetLanguage {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::Dart => Self::Dart,
            LanguageArg::Python => Self::Python,
            LanguageArg::C => Self::C,
            LanguageArg::Cpp => Self::Cpp,
            LanguageArg::JavaScript => Self::JavaScript,
            LanguageArg::TypeScript => Self::TypeScript,
            LanguageArg::CSharp => Self::CSharp,
            LanguageArg::Rust => Self::Rust,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "mavlink-generator",
    about = "Generate MAVLink dialect bindings from XML definitions",
    version,
    subcommand_required = false
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    generate: GenerateArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Generate dialect bindings, runtime helpers, and examples
    Generate(GenerateArgs),
    /// List supported target languages
    ListLanguages,
    /// Parse dialect XML without generating code
    Validate(ValidateArgs),
}

#[derive(Parser)]
struct GenerateArgs {
    /// XML dialect file(s) or a directory to scan for *.xml
    #[arg(short, long = "input", value_name = "PATH")]
    inputs: Vec<PathBuf>,

    /// Output root directory (language folders are created inside)
    #[arg(short, long, default_value = DEFAULT_OUTPUT_ROOT)]
    output: PathBuf,

    /// Target language(s); omit to generate all supported languages
    #[arg(short, long = "lang", value_enum)]
    languages: Vec<LanguageArg>,

    /// Dialect file stem filter when scanning a directory (e.g. rt_rc)
    #[arg(long)]
    dialect: Option<String>,

    /// Include every *.xml file when scanning a directory (ignore dialect filter)
    #[arg(long)]
    all_dialects: bool,

    /// Directory scanned when no --input is given
    #[arg(long, default_value = DEFAULT_DEFINITIONS_DIR)]
    definitions_dir: PathBuf,

    /// Skip runtime helper generation
    #[arg(long)]
    no_runtime: bool,

    /// Skip example generation
    #[arg(long)]
    no_examples: bool,

    /// Suppress progress output
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Parser)]
struct ValidateArgs {
    /// XML dialect file(s) to validate
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

pub fn run_from_args() -> mavlink_generator::Result<()> {
    run(Cli::parse())
}

fn run(cli: Cli) -> mavlink_generator::Result<()> {
    match cli.command {
        Some(Command::Generate(args)) => run_generate(args),
        Some(Command::ListLanguages) => {
            list_languages();
            Ok(())
        }
        Some(Command::Validate(args)) => run_validate(args),
        None => run_generate(cli.generate),
    }
}

fn run_generate(args: GenerateArgs) -> mavlink_generator::Result<()> {
    let languages = selected_languages(&args.languages);
    let xml_paths = resolve_xml_inputs(&args)?;

    for language in languages {
        let language_dir = args.output.join(language.output_dir_name());
        let dialects_dir = language_dir.join("dialects");
        std::fs::create_dir_all(&dialects_dir)?;

        let mut stems_for_language = Vec::with_capacity(xml_paths.len());

        for xml_path in &xml_paths {
            let stem = dialect_stem(xml_path)?;
            stems_for_language.push(stem.clone());

            if !args.quiet {
                println!("[{}] {}", language.display_name(), xml_path.display());
            }

            let dst = dialects_dir.join(format!("{stem}.{}", language.file_extension()));
            generate_code(&dst, xml_path, language)?;

            if !args.quiet {
                println!("  -> {}", dst.display());
            }
        }

        if !args.no_runtime {
            generate_runtime_files(&language_dir, language, &stems_for_language)?;
            if !args.quiet {
                println!(
                    "Generated {} runtime files in {}",
                    language.display_name(),
                    language_dir.display()
                );
            }
        }

        if !args.no_examples {
            generate_example_files(&language_dir, language, &stems_for_language)?;
            if !args.quiet {
                println!(
                    "Generated {} examples in {}",
                    language.display_name(),
                    language_dir.join("examples").display()
                );
            }
        }
    }

    Ok(())
}

fn run_validate(args: ValidateArgs) -> mavlink_generator::Result<()> {
    for input in &args.inputs {
        let document = DialectDocument::parse(input)?;
        let stem = dialect_stem(input)?;
        println!(
            "OK {} (version {}, {} enums, {} messages)",
            stem,
            document.version,
            document.enums.enums().len(),
            document.messages.messages().len()
        );
    }

    Ok(())
}

fn list_languages() {
    for language in all_languages() {
        println!(
            "{} ({})",
            language.display_name(),
            language.output_dir_name()
        );
    }
}

fn selected_languages(languages: &[LanguageArg]) -> Vec<TargetLanguage> {
    if languages.is_empty() {
        return all_languages();
    }

    languages
        .iter()
        .copied()
        .map(TargetLanguage::from)
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

fn resolve_xml_inputs(args: &GenerateArgs) -> mavlink_generator::Result<Vec<PathBuf>> {
    if args.inputs.is_empty() {
        return collect_xml_from_dir(&args.definitions_dir, dialect_filter(args));
    }

    let mut xml_paths = Vec::new();
    for input in &args.inputs {
        if input.is_dir() {
            xml_paths.extend(collect_xml_from_dir(input, dialect_filter(args))?);
        } else {
            xml_paths.push(input.clone());
        }
    }

    if xml_paths.is_empty() {
        return Err(mavlink_generator::GeneratorError::Format(
            "No dialect XML files matched the given inputs".into(),
        ));
    }

    xml_paths.sort();
    xml_paths.dedup();
    Ok(xml_paths)
}

fn dialect_filter(args: &GenerateArgs) -> Option<&str> {
    if args.all_dialects {
        return None;
    }

    Some(args.dialect.as_deref().unwrap_or(DEFAULT_DIALECT_FILTER))
}

fn collect_xml_from_dir(
    dir: &Path,
    dialect_stem_filter: Option<&str>,
) -> mavlink_generator::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(mavlink_generator::GeneratorError::Format(format!(
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
        return Err(mavlink_generator::GeneratorError::Format(format!(
            "No dialect XML files found in {}{filter_note}",
            dir.display()
        )));
    }

    xml_paths.sort();
    Ok(xml_paths)
}

fn dialect_stem(path: &Path) -> mavlink_generator::Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_lowercase)
        .ok_or_else(|| {
            mavlink_generator::GeneratorError::Format(format!(
                "Invalid dialect file name: {}",
                path.display()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_generate_args_match_legacy_behavior() {
        let args = GenerateArgs {
            inputs: vec![],
            output: PathBuf::from(DEFAULT_OUTPUT_ROOT),
            languages: vec![],
            dialect: None,
            all_dialects: false,
            definitions_dir: PathBuf::from(DEFAULT_DEFINITIONS_DIR),
            no_runtime: false,
            no_examples: false,
            quiet: true,
        };

        let xml_paths = resolve_xml_inputs(&args).expect("default inputs should resolve");
        assert_eq!(xml_paths.len(), 1);
        assert!(xml_paths[0].ends_with("rt_rc.xml"));
    }

    #[test]
    fn selected_languages_defaults_to_all() {
        assert_eq!(selected_languages(&[]).len(), 8);
    }
}
