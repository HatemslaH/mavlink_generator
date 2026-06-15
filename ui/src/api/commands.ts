import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface LanguageInfo {
  id: string;
  display_name: string;
  output_dir: string;
}

export interface ValidateResult {
  stem: string;
  version: number;
  enum_count: number;
  message_count: number;
}

export interface GenerateOptions {
  inputs: string[];
  output: string;
  languages: string[];
  dialect: string | null;
  all_dialects: boolean;
  definitions_dir: string;
  runtime: boolean;
  examples: boolean;
}

export interface GenerateProgress {
  stage: string;
  language?: string;
  path?: string;
  message: string;
}

export interface DefaultPaths {
  definitions_dir: string;
  default_output: string;
  default_xml: string;
}

export interface LogLine {
  text: string;
  kind: "info" | "error" | "success";
}

export async function listLanguages(): Promise<LanguageInfo[]> {
  return invoke("list_languages_cmd");
}

export async function defaultPaths(): Promise<DefaultPaths> {
  return invoke("default_paths");
}

export async function validateDialects(
  paths: string[],
): Promise<ValidateResult[]> {
  return invoke("validate_dialects_cmd", { paths });
}

export async function generate(options: GenerateOptions): Promise<void> {
  return invoke("generate", { options });
}

export async function pickXmlFile(): Promise<string | null> {
  return invoke("pick_xml_file");
}

export async function pickOutputDir(): Promise<string | null> {
  return invoke("pick_output_dir");
}

export function listenGenerateProgress(
  handler: (progress: GenerateProgress) => void,
): Promise<UnlistenFn> {
  return listen<GenerateProgress>("generate-progress", (event) => {
    handler(event.payload);
  });
}

export function formatValidateResult(result: ValidateResult): string {
  return `OK ${result.stem} (version ${result.version}, ${result.enum_count} enums, ${result.message_count} messages)`;
}
