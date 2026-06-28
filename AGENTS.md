# AGENTS.md — mavlink_generator

Guide for AI coding agents working in this repository.

## What this project is

**mavlink_generator** is a Rust code generator for [MAVLink](https://mavlink.io/) dialects. It reads MAVLink XML definitions and emits language-specific bindings (message types, enums, runtime helpers, and examples).

Two delivery surfaces share the same engine:

| Surface | Location | Entry |
|---------|----------|-------|
| CLI | `src/main.rs` | `cargo run --bin mavlink-generator` |
| Desktop UI | `ui/` (Tauri 2 + React 19) | `cd ui && pnpm tauri dev` |

The shared orchestration layer is `src/driver.rs`. The UI calls it through Tauri commands in `ui/src-tauri/src/lib.rs`.

## Repository layout

```
src/
  driver.rs          # CLI + UI orchestration (GenerateOptions, validate, run_generate)
  error.rs           # GeneratorError, Result<T>
  xml/               # MAVLink XML parser → DialectDocument
  generate/          # Per-language dialect, runtime, and example generators
    mod.rs           # TargetLanguage enum, LanguageGenerator trait
    runtime.rs       # Runtime file generation
    examples.rs      # Example file generation
    <lang>/          # dart, python, c, cpp, typescript, csharp, rust, javascript
templates/<lang>/    # Static runtime templates copied/adapted during generation
generated/           # Generator output (do not hand-edit; regenerate instead)
mavlink/             # Upstream MAVLink definitions (git submodule)
tests/generator.rs   # Integration tests for parsing and generation
ui/                  # Tauri desktop app
  src/               # React frontend
  src-tauri/         # Tauri backend (depends on root crate via path)
```

## Commands agents should run

### Rust core

```bash
cargo fmt
cargo check
cargo test
```

Run from the repository root. Integration tests in `tests/generator.rs` exercise parsing and multi-language generation.

### Desktop UI

```bash
cd ui
pnpm install
pnpm build          # typecheck + vite build
pnpm tauri dev      # development
pnpm tauri build    # release installer
```

Prerequisites: Node.js 20+, pnpm 9+, Rust, Tauri system deps (WebView2 on Windows).

### Regenerate bindings (smoke test)

```bash
cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang rust
```

## Architecture notes

### Generation pipeline

1. **Parse** — `DialectDocument::parse(path)` in `src/xml/`
2. **Dialect** — `generate_dialect()` renders message/enum code per language
3. **Runtime** — `generate_runtime_files()` writes shared helpers from `templates/<lang>/`
4. **Examples** — `generate_example_files()` writes runnable samples from `templates/<lang>/examples/`
5. **Entry point** — each language gets a root file (`mavlink.dart`, `lib.rs`, etc.) that exports dialects + runtime

### Adding a target language

Follow the checklist in `README.md` → **Extending → Add a target language**:

1. Add `TargetLanguage` variant in `src/generate/mod.rs`
2. Implement `src/generate/<language>/` (dialect `render`, `LanguageRuntimeGenerator`, `LanguageExampleGenerator`)
3. Add templates under `templates/<language>/`
4. Register in `runtime.rs` and `examples.rs`
5. Add tests in `tests/generator.rs`

### UI ↔ Rust bridge

- Frontend API: `ui/src/api/commands.ts` — thin `invoke()` wrappers
- Backend commands: `ui/src-tauri/src/lib.rs`
- Shared types: `GenerateOptions`, `ValidateResult`, `GenerateProgress` from `driver.rs` (serde)
- Long-running `generate` runs on `spawn_blocking`; progress via `generate-progress` events

## Boundaries — do not modify

| Path | Reason |
|------|--------|
| `mavlink/` (upstream) | Git submodule; vendor definitions only, no local patches |
| `generated/` | Output of the generator; change templates or generators instead |
| `private/` | Personal notes; gitignored |
| `ui/node_modules/`, `target/`, `ui/src-tauri/target/` | Build artifacts |

## Versioning and releases

Version must stay in sync across:

- `Cargo.toml` (root crate)
- `ui/package.json`
- `ui/src-tauri/tauri.conf.json`

Releases are tag-driven (`v*`) via `.github/workflows/release.yml`. Release notes go in `docs/release-notes/`.

## Conventions

- **Rust edition**: 2024 (root crate), 2021 (Tauri crate)
- **Errors**: use `GeneratorError` / `Result<T>`; avoid `unwrap()` outside tests
- **Scope**: minimal diffs; match existing module layout per language
- **Tests**: add integration tests in `tests/generator.rs` for generation changes; assert on file content substrings
- **Commits**: only when explicitly requested by the user
- **Language**: README and user docs are English; agent may respond in the user's language

## Common tasks

| Task | Where to look |
|------|---------------|
| Fix XML parsing | `src/xml/` |
| Fix generated Dart/Python/… output | `src/generate/<lang>/` + `templates/<lang>/` |
| Add CLI flag | `src/main.rs` + `src/driver.rs` |
| UI feature | `ui/src/` + `ui/src-tauri/src/lib.rs` |
| CI / release | `.github/workflows/` |

## Cursor rules

Project-specific agent rules live in `.cursor/rules/`:

- `project-core.mdc` — always-on project context
- `rust-generator.mdc` — Rust source conventions
- `code-generation.mdc` — generators and templates
- `tauri-ui.mdc` — desktop UI development
