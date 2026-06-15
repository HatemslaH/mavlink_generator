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
      ...
  py/
    dialects/
      rt_rc.py
    crc.py
    mavlink.py         # entry point (imports dialects + runtime)
    ...
    examples/
      README.md
      common.py
      rt_rc_heartbeat.py
      ...
  c/
    dialects/
      rt_rc.h
    crc.h
    mavlink.h          # entry point (includes dialects + runtime)
    ...
    examples/
      README.md
      common.h
      rt_rc_heartbeat.c
      ...
  cpp/
    dialects/
      rt_rc.hpp
    crc.hpp
    mavlink.hpp        # entry point (includes dialects + runtime)
    ...
    examples/
      README.md
      common.hpp
      rt_rc_heartbeat.cpp
      ...
  ts/
    dialects/
      rt_rc.ts
    crc.ts
    mavlink.ts         # entry point (exports dialects + runtime)
    ...
    examples/
      README.md
      common.ts
      rt_rc_heartbeat.ts
      ...
  csharp/
    dialects/
      rt_rc.cs
    crc.cs
    mavlink.cs         # entry point (includes dialects + runtime)
    ...
    examples/
      README.md
      common.cs
      rt_rc_heartbeat.cs
      ...
  rust/
    dialects/
      rt_rc.rs
    crc.rs
    lib.rs             # entry point (exports dialects + runtime)
    Cargo.toml
    ...
    examples/
      README.md
      common.rs
      rt_rc_heartbeat.rs
      ...
  js/
    dialects/
      rt_rc.js
    crc.js
    mavlink.js         # entry point (exports dialects + runtime)
    package.json
    ...
    examples/
      README.md
      common.js
      rt_rc_heartbeat.js
      ...
```

Input definitions live in `mavlink/message_definitions/`. Static runtime templates live in `templates/<language>/`.

## Requirements

- Rust 1.85+ (edition 2024)
- MAVLink XML dialect files (included as a git submodule or vendored copy under `mavlink/`)

To run generated examples:

- **Dart** — Dart SDK
- **Python** — Python 3.10+ (generated dialects use `match`)
- **C** — C compiler with C11 support (e.g. gcc, clang)
- **C++** — C++17 compiler (e.g. g++, clang++)
- **TypeScript** — Node.js with [tsx](https://github.com/privatenumber/tsx) or similar
- **C#** — .NET SDK or `csc` compiler
- **Rust** — Rust toolchain (generated crate uses edition 2021)
- **JavaScript** — Node.js

## Usage

### CLI

Build the generator:

```bash
cargo build --release
```

Run with defaults (all languages, `rt_rc` dialect, output in `generated/`):

```bash
cargo run
# or after build:
cargo run --release --bin mavlink-generator
```

Common options:

```bash
# One dialect, selected languages
cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang rust --lang python

# Scan a definitions directory for every dialect XML
cargo run -- generate --definitions-dir mavlink/message_definitions/v1.0 --all-dialects --lang dart

# Dialect + runtime only (skip examples)
cargo run -- --input mavlink/message_definitions/v1.0/rt_rc.xml --lang rust --no-examples

# Validate XML without generating code
cargo run -- validate mavlink/message_definitions/v1.0/rt_rc.xml

# List supported target languages
cargo run -- list-languages
```

Flags:

| Flag | Description |
|------|-------------|
| `--input`, `-i` | XML file or directory (repeatable) |
| `--output`, `-o` | Output root (default: `generated/`) |
| `--lang`, `-l` | Target language (repeatable; default: all) |
| `--dialect` | Stem filter when scanning a directory (default: `rt_rc`) |
| `--all-dialects` | Include every `*.xml` in a scanned directory |
| `--definitions-dir` | Directory used when `--input` is omitted |
| `--no-runtime` | Skip runtime helper generation |
| `--no-examples` | Skip example generation |
| `--quiet`, `-q` | Suppress progress output |

Install locally:

```bash
cargo install --path .
mavlink-generator --help
```

### Run examples

Each language ships four virtual examples per dialect (heartbeat, mission upload, telemetry request, parameter request). See `generated/<language>/examples/README.md` for details.

**Dart** (from `generated/dart`):

```bash
dart run examples/rt_rc_heartbeat.dart
```

**Python** (from `generated/py`):

```bash
python examples/rt_rc_heartbeat.py
```

**C** (from `generated/c`):

```bash
gcc -std=c11 -I. examples/rt_rc_heartbeat.c -o rt_rc_heartbeat
./rt_rc_heartbeat
```

**C++** (from `generated/cpp`):

```bash
g++ -std=c++17 -I. examples/rt_rc_heartbeat.cpp -o rt_rc_heartbeat
./rt_rc_heartbeat
```

**TypeScript** (from `generated/ts`):

```bash
npx tsx examples/rt_rc_heartbeat.ts
```

**C#** (from `generated/csharp`):

```bash
dotnet run --project examples/rt_rc_heartbeat.csproj
```

**Rust** (from `generated/rust`):

```bash
cargo run --example rt_rc_heartbeat
```

**JavaScript** (from `generated/js`):

```bash
node examples/rt_rc_heartbeat.js
```

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
// TargetLanguage::Python, TargetLanguage::C, TargetLanguage::Cpp,
// TargetLanguage::TypeScript, TargetLanguage::CSharp, TargetLanguage::Rust,
// TargetLanguage::JavaScript
```

## Supported languages

| Language   | Output dir | Dialect generation | Runtime generation | Examples |
|------------|------------|-------------------|-------------------|----------|
| Dart       | `dart/`    | yes               | yes               | yes      |
| Python     | `py/`      | yes               | yes               | yes      |
| C          | `c/`       | yes               | yes               | yes      |
| C++        | `cpp/`     | yes               | yes               | yes      |
| TypeScript | `ts/`      | yes               | yes               | yes      |
| C#         | `csharp/`  | yes               | yes               | yes      |
| Rust       | `rust/`    | yes               | yes               | yes      |
| JavaScript | `js/`      | yes               | yes               | yes      |

## Project layout

```
src/
  xml/                 # MAVLink XML parser
  generate/
    dart/              # Dart dialect + runtime + examples generator
    python/            # Python dialect + runtime + examples generator
    c/                 # C dialect + runtime + examples generator
    cpp/               # C++ dialect + runtime + examples generator
    typescript/        # TypeScript dialect + runtime + examples generator
    csharp/            # C# dialect + runtime + examples generator
    rust/              # Rust dialect + runtime + examples generator
    javascript/        # JavaScript dialect + runtime + examples generator
    runtime.rs         # shared output paths and runtime trait
    examples.rs        # shared output paths and examples trait
  main.rs              # CLI entry point
templates/
  dart/                # static runtime templates
  py/
  c/
  cpp/
  ts/
  csharp/
  rust/
  js/
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
