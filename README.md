# mavlink_generator

Code generator for [MAVLink](https://mavlink.io/) dialects. Reads MAVLink XML definitions and produces language-specific message types, enums, and runtime helpers.

## Overview

The generator has three outputs per target language:

1. **Dialect files** — message classes, enums, and a dialect registry derived from XML.
2. **Runtime files** — shared helpers (CRC, framing, parsing) that dialect code depends on.
3. **Examples** — runnable sample code showing how to use the generated bindings.

All outputs are written under a common layout so multiple languages and dialects can coexist:

```
generated/
  dart/
    dialects/          # one file per dialect
      rt_rc.dart
    crc.dart           # runtime helpers
    mavlink.dart       # entry point (exports dialects + runtime)
    ...
    examples/          # usage examples (one per dialect + common helpers)
      README.md
      common.dart
      rt_rc_heartbeat.dart
      rt_rc_mission_upload.dart
      rt_rc_request_telemetry.dart
      rt_rc_request_parameters.dart
  py/                  # reserved
  c/                   # reserved
```

Input definitions live in `mavlink/message_definitions/`. Static runtime templates live in `templates/<language>/`.

## Requirements

- Rust 1.85+ (edition 2024)
- MAVLink XML dialect files (included as a git submodule or vendored copy under `mavlink/`)

## Usage

### CLI

Build and run the generator:

```bash
cargo build --release
cargo run
```

By default the CLI generates Dart output for configured dialects into `generated/dart/`.

### Library

```rust
use mavlink_generator::{TargetLanguage, generate_dialect, generate_example_files, generate_runtime_files, language_output_dir};

// Generate a single dialect
generate_dialect("mavlink/message_definitions/v1.0/rt_rc.xml", TargetLanguage::Dart, "rt_rc")?;

// Generate runtime files and entry point
let output = language_output_dir(TargetLanguage::Dart);
generate_runtime_files(&output, TargetLanguage::Dart, &["rt_rc".into()])?;

// Generate usage examples
generate_example_files(&output, TargetLanguage::Dart, &["rt_rc".into()])?;
```

Lower-level API:

```rust
use mavlink_generator::{TargetLanguage, generate_code};

generate_code("out/custom.dart", "path/to/dialect.xml", TargetLanguage::Dart)?;
```

## Supported languages

| Language | Dialect generation | Runtime generation | Examples |
|----------|-------------------|-------------------|----------|
| Dart     | yes               | yes               | yes      |
| Python   | planned           | planned           | planned  |
| C        | planned           | planned           | planned  |

## Project layout

```
src/
  xml/                 # MAVLink XML parser
  generate/
    dart/              # Dart dialect + runtime generator
    python/            # Python dialect generator (stub)
    c/                 # C dialect generator (stub)
    runtime.rs         # shared output paths and runtime trait
    examples.rs        # shared output paths and examples trait
  main.rs              # CLI entry point
templates/
  dart/                # static runtime templates
  py/
  c/
generated/             # output (created by the generator)
mavlink/               # MAVLink definitions (upstream)
tests/
```

## Extending

### Add a dialect

Place an XML file under `mavlink/message_definitions/v1.0/` and call `generate_dialect` with the file stem as the dialect name.

### Add a target language

1. Add a variant to `TargetLanguage` in `src/generate/mod.rs`.
2. Implement dialect rendering in `src/generate/<language>/` (`LanguageGenerator` / `render` function).
3. Add runtime templates under `templates/<language>/`.
4. Implement `LanguageRuntimeGenerator` and register it in `src/generate/runtime.rs`.
5. Add example templates under `templates/<language>/examples/` and implement `LanguageExampleGenerator` in `src/generate/examples.rs`.
6. Add tests in `tests/generator.rs`.

Dialect generators produce message-specific code. Runtime generators produce language-wide helpers and an entry-point file that exports all generated dialects. Example generators produce runnable sample programs per dialect.

## Development

```bash
cargo fmt
cargo check
cargo test
```

## License

See repository license file.
