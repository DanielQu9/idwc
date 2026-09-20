# IdwC Stupid Mode

[繁體中文](stupid-mode.zh-TW.md) · [Back to README](../README.md) ·
[Strict semantics](strict-semantics.md)

Stupid Mode is the optional readable-C generator introduced in IdwC v1.1.0.
Enable it with `-s` / `--stupid`, or select [`TranspileMode::Stupid`] through
the library API. Strict mode remains the default.

```bash
idwc --stupid input.rs -o output.c
```

The name is a joke; the tradeoff is deliberate. Stupid Mode accepts the same
validated Rust subset as strict mode, but prioritizes concise, editable C over
Rust-equivalent runtime behavior.

## Generated source

- Source variable, parameter, range-binding, and function names are retained
  when they are legal and unambiguous in C.
- C keywords, reserved identifiers, standard-library collisions, and Rust
  shadowing receive deterministic suffixes such as `value_2`.
- Visible Unicode is written directly in C17 `u8"..."` literals. Quotes,
  backslashes, control characters, question marks, and `printf` percent signs
  are still escaped where required.
- Only required headers are included. IdwC emits no private runtime helper
  function or struct definition. Functions present in the Rust source remain
  ordinary C functions.
- `i32`, `usize`, `f64`, and `bool` map to `int`, `size_t`, `double`, and
  `bool`. Fixed arrays remain local C arrays. A supported `Vec<T>` becomes one
  local C array plus a readable length variable such as `values_len`; no Vec
  struct or helper function is emitted.
- Limited collection `{:?}` output becomes an ordinary C `for` loop that writes
  brackets and separators. The shared semantic validator still rejects `f64`
  collection Debug formatting.
- Mutable `&str` bindings remain `const char *`: Rust binding mutability permits
  reassignment of the pointer, not mutation of string-literal bytes.
- Simple Vec `push`, repeat values, and index assignment emit direct C writes.
  Reusable mixed-format arguments also stay inline. A side-effecting right-hand
  side is evaluated into a temporary before the index expression; moving one
  Vec binding into another uses one explicit copy loop.

## Deliberately omitted guarantees

Stupid Mode emits direct C arithmetic and indexing. It does not check signed
overflow, negation overflow, division or remainder by zero, the `INT_MIN / -1`
case, unsigned wraparound, array bounds, or inclusive-range overflow. Any
resulting C undefined behavior is part of selecting this mode.

Vec capacity, `push`, and indexing are likewise unchecked. Exceeding the
configured local array or indexing past the logical length can invoke C
undefined behavior.

Function arguments and other direct C expressions may follow C evaluation
order instead of Rust left-to-right order. A temporary is retained only when
valid C or avoiding an obvious duplicate side effect requires one.

Floating-point values use ordinary `double` expressions. IdwC does not set the
floating-point environment, reject fast-math, force per-operation rounding, or
provide Rust-compatible `Display`. `{}` uses `%g`; fixed precision uses C
`%.Nf`. Locale, rounding, special values, overflow, underflow, and excess
precision follow the C implementation and compiler options.

## Input behavior

- `read_i32()` and `read_f64()` become unchecked `%d` and `%lf` `scanf` calls.
  A failed conversion can leave an uninitialized value.
- `flush_stdout()` becomes a direct `fflush(stdout)` call with no result check.
- The limited `String` input path uses a local `char` array with one NUL byte
  beyond the configured payload capacity (4096 by default) and unchecked
  `fgets`. `idwc::io::read_line()` lowers directly to an empty array followed
  by one such call.
- Token collections use the configured Vec capacity (2048 by default) without
  an overflow check.
- `&str` bindings become `char` pointers. Owned String values become local
  arrays; `String::from`, assignment, and moves use unchecked `strcpy`, while
  `{}` output uses `%s`. Capacity overflow and embedded NUL behavior therefore
  follow C rather than strict mode.
- `split_whitespace()` uses `strtok` with ASCII whitespace and a local pointer
  array sized for every possible token in the fixed line buffer. Unicode
  whitespace behavior is not preserved.
- `parse::<i32>()` and `parse::<f64>()` use unchecked `strtol` and `strtod`.
  Token indexing is unchecked.

Stupid Mode is intended for small, controlled classroom programs with known
input. It is unsuitable for untrusted input or code that depends on Rust panic,
overflow, floating-point, bounds, UTF-8, or evaluation-order guarantees.

## Checks that remain

Both modes use `syn`, the supported-AST whitelist, name and scope resolution,
mutability validation, and type checking. Unsupported syntax, undefined names,
and invalid type combinations are rejected before C generation. Stupid Mode
does not execute input Rust or expand user-defined macros.

The CLI writes one warning to stderr when the mode is selected. The generated
file also contains a short comment identifying the relaxed contract.

[`TranspileMode::Stupid`]: https://docs.rs/idwc/latest/idwc/enum.TranspileMode.html#variant.Stupid
