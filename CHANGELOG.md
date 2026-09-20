# Changelog

All notable changes to IdwC are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
