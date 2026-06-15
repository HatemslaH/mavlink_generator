use std::path::PathBuf;

use mavlink_generator::{
    GenerateOptions, GenerateProgress, LanguageInfo, ValidateResult, DEFAULT_DEFINITIONS_DIR,
    DEFAULT_OUTPUT_ROOT, list_languages, run_generate, validate_dialects,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
struct DefaultPaths {
    definitions_dir: String,
    default_output: String,
    default_xml: String,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

#[tauri::command]
fn list_languages_cmd() -> Vec<LanguageInfo> {
    list_languages()
}

#[tauri::command]
fn default_paths() -> DefaultPaths {
    let workspace = workspace_root();
    let definitions_dir = workspace.join(DEFAULT_DEFINITIONS_DIR);
    DefaultPaths {
        definitions_dir: definitions_dir.display().to_string(),
        default_output: workspace.join(DEFAULT_OUTPUT_ROOT).display().to_string(),
        default_xml: definitions_dir.join("rt_rc.xml").display().to_string(),
    }
}

#[tauri::command]
fn validate_dialects_cmd(paths: Vec<String>) -> Result<Vec<ValidateResult>, String> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    validate_dialects(&paths).map_err(|error| error.to_string())
}

#[tauri::command]
async fn generate(app: AppHandle, options: GenerateOptions) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_generate(&options, |progress: GenerateProgress| {
            let _ = app.emit("generate-progress", &progress);
        })
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn pick_xml_file(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter("MAVLink XML", &["xml"])
        .blocking_pick_file()
        .map(|path| path.to_string())
}

#[tauri::command]
async fn pick_output_dir(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|path| path.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_languages_cmd,
            default_paths,
            validate_dialects_cmd,
            generate,
            pick_xml_file,
            pick_output_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
