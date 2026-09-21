# Changelog

All notable changes to IdwC are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Stupid Mode omits redundant `(void)` Vec warning suppressions when generated
  C later reads the storage or length, while retaining them for truly unused
  objects so warning-as-error builds still compile.
- Planned a clap-managed, locale-aware bilingual CLI for v1.3.0 and shifted the
  subsequent collection, String, iterator, and pattern milestones accordingly.

## [1.2.1] - 2026-09-21

### Changed

- Stupid Mode now emits direct writes for simple Vec `push`, repeat values, and
  index assignment; reusable mixed-format arguments stay inline, and Vec moves
  copy in one loop without a full temporary array.
- Documented the planned collection, bounded String, eager `.map`, short
  iterator-pipeline, and pattern-based control-flow milestones for the next
  minor releases.

### Fixed

- Stupid Mode preserves `const char *` for mutable `&str` bindings and avoids
  invalid duplicate `const` specifiers when string and collection formatting
  appear in the same output macro.

## [1.2.0] - 2026-09-21

### Added

- `idwc::io::read_line() -> String` for one-call bounded UTF-8 line input in
  native Rust, strict generated C, and Stupid Mode.
- `--string-capacity` / `--vec-capacity` CLI options and matching
  `TranspileOptions` builders for generated stack capacities.
- String-literal `&str` bindings and bounded owned `String` values with
  `String::new`, `String::from`, assignment, local move checking, and `{}`
  output.
- Fixed-capacity `Vec<i32|usize|f64|bool>` values with `Vec::new`, both `vec!`
  forms, `push`, indexing, `.len()`, assignment, and local move checking.
- Limited `{:?}` output for `i32`, `usize`, and `bool` fixed arrays and Vecs,
  lowered directly to generated C loops.
- Strict Vec capacity and index failures with status 101, plus readable local
  C array and length-variable output in Stupid Mode.

### Changed

- The default `Vec<&str>` token capacity is now 2048 elements, matching the
  planned fixed-capacity Vec default.

## [1.1.1] - 2026-09-20

### Added

- Detailed `--help` output describing arguments, options, modes, and examples.
- Optional `-o`: when omitted, the CLI replaces the input `.rs` extension
  with `.c` for the output path.

### Documentation

- Consolidated limited string values, `vec!`, a fixed-capacity scalar `Vec`
  subset, and direct-loop collection output into the v1.2.0 plan.
- Planned `idwc::io::read_line() -> String` as the v1.2.0 convenience API for
  bounded UTF-8 line input.

## [1.1.0] - 2026-09-20

### Added

- Optional `-s` / `--stupid` CLI generation for concise, editable C.
- `TranspileMode`, `TranspileOptions`, and `transpile_with_options` library API;
  the existing `transpile` entry point remains strict.
- Direct UTF-8 `u8` C string literals and predictable preservation of source
  function, parameter, variable, and range-binding names in Stupid Mode.
- Bilingual Stupid Mode contracts and C17 compilation coverage for every Rust
  example.

### Changed

- Stupid Mode uses direct C arithmetic, indexing, floating-point formatting,
  and standard-library input without generated runtime helpers, structs, or
  semantic error checks.
- Updated the crate, CLI, documentation, and version tests to 1.1.0.

## [1.0.0] - 2026-09-20

### Added

- crates.io package metadata, installation instructions, and a repeatable
  release checklist.
- A documented compatibility contract for the public Rust API, strict-mode
  behavior, runtime diagnostics, and generated C text.

### Changed

- Declared the tested Rust subset and bilingual strict semantic specification
  stable for the 1.x release line.
- Updated the crate, CLI, documentation, and version tests to 1.0.0.
- Made zero-length array bounds failures warning-free under strict GCC and
  documented the intentional Clippy exception for supported format syntax.

## [0.9.0] - 2026-09-20

### Added

- Stable `TranspileErrorKind` categories and public message/location accessors.
- One-based source locations for parse, validation, and semantic diagnostics
  when an associated syntax node is available.

### Changed

- `TranspileError` is now an opaque diagnostic structure instead of a public
  enum, finalizing the pre-1.0 library error API without exposing internal
  payloads.
- Release verification now covers the public diagnostics, locked package
  metadata, strict C17 builds, Clang/GCC CI, and bilingual semantic contracts.

## [0.8.0] - 2026-09-20

### Added

- Strict translation of `i32` and `usize` `for` loops over `start..end` and
  `start..=end` integer ranges.
- Empty `print!()` and `println!()` calls.
- `--version` and `-V` CLI options.
- English and Traditional Chinese documentation with separate strict semantic
  contracts.
- Checked-in C example consistency tests and Clang/GCC CI coverage.

### Changed

- CLI output uses a same-directory temporary file and atomic rename so a failed
  write cannot leave a partially generated C file.
- `TranspileError` is non-exhaustive so future diagnostic variants do not break
  downstream exhaustive matches.

## [0.7.0] - 2026-09-20

### Added

- Limited `String::new()`, `read_line`, `trim`, `split_whitespace`,
  `Vec<&str>`, `parse::<i32>()`, and `parse::<f64>()` input pipeline.
- Limited `f64::powi(2)` translation.
- UTF-8, Unicode whitespace, input-buffer, token-count, and token-index checks.

## [0.6.0]

### Added

- Fixed-size arrays, `usize` indices, value-copy semantics, and bounds checks.

## [0.5.0]

### Added

- Strict `f64` arithmetic, input, output, special-value, and rounding behavior.

## [0.4.0]

### Added

- Functions, typed integer input, interactive output, and explicit flushing.

## [0.3.0]

### Added

- `if` / `else`, `while`, `loop`, `break`, and `continue`.

## [0.2.0]

### Added

- Variables, scalar types, expressions, assignment, scopes, and checked integer
  arithmetic.

## [0.1.0]

### Added

- End-to-end translation of a minimal `println!("Hello, World!")` program.
