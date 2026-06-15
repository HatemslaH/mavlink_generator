use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use mavlink_generator::driver::{
    self, DEFAULT_DEFINITIONS_DIR, DEFAULT_OUTPUT_ROOT, GenerateOptions, GenerateProgress,
};

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

impl LanguageArg {
    fn to_id(self) -> &'static str {
        match self {
            Self::Dart => "dart",
            Self::Python => "py",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::CSharp => "csharp",
            Self::Rust => "rust",
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

impl From<GenerateArgs> for GenerateOptions {
    fn from(args: GenerateArgs) -> Self {
        Self {
            inputs: args.inputs,
            output: args.output,
            languages: args
                .languages
                .into_iter()
                .map(|language| language.to_id().to_string())
                .collect(),
            dialect: args.dialect,
            all_dialects: args.all_dialects,
            definitions_dir: args.definitions_dir,
            runtime: !args.no_runtime,
            examples: !args.no_examples,
        }
    }
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
    let quiet = args.quiet;
    let options = GenerateOptions::from(args);
    driver::run_generate(&options, |progress| {
        if !quiet {
            print_progress(&progress);
        }
    })
}

fn run_validate(args: ValidateArgs) -> mavlink_generator::Result<()> {
    for result in driver::validate_dialects(&args.inputs)? {
        println!(
            "OK {} (version {}, {} enums, {} messages)",
            result.stem, result.version, result.enum_count, result.message_count
        );
    }

    Ok(())
}

fn list_languages() {
    for language in driver::list_languages() {
        println!("{} ({})", language.display_name, language.output_dir);
    }
}

fn print_progress(progress: &GenerateProgress) {
    println!("{}", progress.message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink_generator::driver;

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

        let options = GenerateOptions::from(args);
        let xml_paths = driver::resolve_inputs(&options).expect("default inputs should resolve");
        assert_eq!(xml_paths.len(), 1);
        assert!(xml_paths[0].ends_with("rt_rc.xml"));
    }
}
