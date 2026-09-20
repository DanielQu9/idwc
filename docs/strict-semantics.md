# IdwC strict semantics

[繁體中文](strict-semantics.zh-TW.md) · [Back to README](../README.md)

This document defines the strict translation contract for IdwC v1.0.0.
Anything not listed here is rejected. The generated program aims to preserve
the behavior of the accepted Rust subset without relying on C undefined
behavior.

## Program and functions

- A file contains functions only and has exactly one private `fn main()`.
- `main` has no parameters, modifiers, attributes, generics, or explicit return
  type.
- Additional private functions may use `i32`, `usize`, `f64`, or `bool` scalar
  parameters and scalar or unit returns. `mut` parameters are accepted.
- Forward calls and recursion are supported. Calling `main`, indirect calls,
  methods other than the listed built-ins, and user-defined generics are not.
- `return;`, `return ();`, typed `return value;`, and a final scalar expression
  are accepted. A conservative analysis requires every non-unit function to
  have a guaranteed return.
- Parameters are copied. Every function has independent lexical scopes.

## Bindings and types

- Bindings use initialized `let` or `let mut` with a single ASCII identifier.
- Supported scalar types are `i32`, restricted `usize`, `f64`, and `bool`.
- Unsuffixed integers default to `i32`, except in a required `usize` context.
  `i32` and `usize` suffixes are accepted.
- Decimal, hexadecimal, octal, and binary integer literals are accepted.
  `-2147483648` is accepted; associated paths such as `i32::MIN` are not.
- Decimal and scientific floating literals default to `f64`; the `f64` suffix
  and underscores are accepted. `f32` and nondecimal floats are rejected.
- Same-scope and nested shadowing are supported. Undefined names, values used
  outside their scope, immutable assignment, and type mismatches are errors.
- General String and Vec values are not supported. Their limited input-only
  forms are defined below.

## Expressions and evaluation

- Matching numeric operands support `+`, `-`, `*`, `/`, `<`, `<=`, `>`, and
  `>=`. `i32` and `usize` additionally support `%`.
- `==` and `!=` support matching scalar types. Mixed numeric types are rejected.
- Unary `-` supports `i32` and `f64`; `!` supports `bool`.
- `&&` and `||` short-circuit. Other binary operands and function arguments are
  evaluated from left to right by generated temporaries.
- Plain and arithmetic compound assignment are accepted for mutable bindings.
  Assignment is a statement, not a value expression.
- `f64::powi(2)` is the only supported numerical method. The receiver is
  evaluated once and squared using one binary64 multiplication.
- Scalar expression statements may discard their result while retaining checks
  and side effects. Value-producing blocks are not supported.

## Control flow

- Statement-form `if` / `else if` / `else` requires `bool` conditions and unit
  branches. Only the selected branch executes.
- Unlabeled `while` and `loop` are supported. A `while` condition is evaluated
  before every iteration, including after `continue`.
- Unlabeled `break` and `continue` affect the innermost loop and cannot carry a
  value.
- Integer range loops accept `for binding in start..end` and
  `for binding in start..=end` with matching `i32` or `usize` bounds.
- Range bounds are evaluated once, from start to end, before iteration.
- The range binding has a new lexical scope and is recreated from an independent
  iterator value on every iteration. A `mut` binding may be changed without
  changing which value the iterator yields next.
- Empty and descending ranges execute no iterations. Inclusive ranges stop
  without incrementing past their endpoint, including `i32::MAX` or
  `usize::MAX`. `break` and `continue` retain Rust behavior.

Range values outside a `for`, open-ended ranges, noninteger ranges, labels,
general iterators, `.rev()`, `if let`, `while let`, `match`, and value-producing
control-flow expressions are rejected.

## Fixed arrays

- Local one-dimensional `[T; N]` supports `T = i32`, `usize`, `f64`, or `bool`.
- `N` is an integer literal from 0 through 4096.
- Array literals, `[value; N]`, whole-array copy and assignment, `.len()`, and
  named-binding element reads/writes are supported.
- Indices are `usize`, evaluated once, and checked before C storage is touched.
- Zero-length arrays use one inaccessible physical C element while retaining a
  logical length of zero.
- Bounds failure flushes stdout, writes `idwc: array index out of bounds` to
  stderr, and exits with status 101.
- Nested arrays, slices, references, array parameters/returns, iterator methods,
  array equality, and indexing temporary array expressions are rejected.

## Output

- Empty `print!()` and `println!()` are accepted.
- Other calls require a string literal followed by sequential arguments.
- `{}` accepts `i32`, `usize`, `f64`, and `bool`. `{:.0}` through `{:.18}`
  accept `f64`. `{{` and `}}` produce literal braces.
- Arguments are evaluated left to right before any part of a formatted message
  is written.
- Text-only `println!` uses `puts`; text-only `print!` uses `fputs`.
- Input text never becomes a C format string. Embedded NUL is rejected. UTF-8,
  control bytes, and question marks are escaped safely for C17.
- Captured, numbered, named, debug, width, and other format forms are rejected.
- stdout I/O failure behavior is not guaranteed to match Rust, except for the
  explicit flush contract below.

## Typed token input

`idwc::io::read_i32()` and `idwc::io::read_f64()` read whitespace-separated
tokens. `idwc::io::flush_stdout()` supports interactive prompts.

- Whitespace is ASCII space, tab, LF, CR, vertical tab, or form feed.
- A token contains at most 128 bytes, including its sign.
- `read_i32` accepts `[+-]?[0-9]+` and checks the complete i32 range.
- `read_f64` accepts decimal/scientific syntax plus exactly `NaN`, `inf`,
  `+inf`, and `-inf`.
- Float overflow to infinity and nonzero underflow to zero are range errors for
  this typed-token API. Representable subnormals and signed zero are accepted.
- Integer and float reads may alternate. A valid token ending at EOF succeeds;
  the next read reports EOF.

Failures flush stdout, write the listed diagnostic, and exit with status 101:

| Condition | Diagnostic |
| --- | --- |
| EOF before a token | `idwc: unexpected EOF` |
| stdin error | `idwc: stdin I/O error` |
| Invalid integer | `idwc: invalid integer` |
| Integer outside i32 | `idwc: integer out of range` |
| Invalid float | `idwc: invalid float` |
| Float overflow or nonzero underflow to zero | `idwc: float out of range` |
| More than 128 token bytes | `idwc: input token too long` |
| Explicit stdout flush failure | `idwc: stdout flush error` |

## Limited line input

The line path accepts the following local pattern:

```rust
let mut input = String::new();
std::io::stdin().read_line(&mut input).unwrap();
let values: Vec<&str> = input.split_whitespace().collect();
let number = values[0].parse::<f64>().unwrap();
```

- A generated String has 4096 bytes of stack storage. `read_line` appends,
  retains the newline, and succeeds without modification on empty EOF.
- Accumulated content may not exceed 4096 bytes and must be valid UTF-8 after
  every read.
- `trim()` and `split_whitespace()` use Rust-compatible Unicode White_Space.
- A collected `Vec<&str>` is at most 256 stack-allocated spans borrowing the
  source buffer. It does not allocate or copy token text.
- Another `read_line` is conservatively rejected while a token collection from
  the same String remains in lexical scope.
- `input.trim().parse::<i32|f64>().unwrap()` and
  `tokens[index].parse::<i32|f64>().unwrap()` are accepted. Token indices are
  evaluated once and bounds checked. Collections also expose `.len()`.
- String `parse::<f64>()` follows Rust range results: overflow becomes signed
  infinity and nonzero underflow may become signed zero. This differs from the
  typed-token `read_f64` range policy.

Line-runtime failures use status 101:

| Condition | Diagnostic |
| --- | --- |
| stdin error | `idwc: stdin I/O error` |
| Invalid UTF-8 | `idwc: invalid UTF-8 input` |
| More than 4096 accumulated bytes | `idwc: input line buffer too long` |
| More than 256 tokens | `idwc: too many input tokens` |
| Token index out of bounds | `idwc: token index out of bounds` |
| Invalid/out-of-range integer | `idwc: invalid integer` / `idwc: integer out of range` |
| Invalid float | `idwc: invalid float` |

These `.unwrap()` forms use controlled failure rather than reproducing Rust
panic text or unwinding. General allocation, String/Vec assignment, cloning,
concatenation, function parameters/returns, arbitrary methods, slices,
iterators, and ownership are rejected.

## Integer semantics

- `i32` maps to `int32_t`. Addition, subtraction, multiplication, and negation
  compute in `int64_t`, check the i32 range, then narrow.
- Division and remainder reject zero and the `INT32_MIN / -1` overflow case.
- Arithmetic failures flush stdout, write a diagnostic, and exit with 101.
- Arithmetic remains checked at every C optimization level, corresponding to
  Rust compiled with `-C overflow-checks=yes` rather than release wrapping.
- Restricted `usize` maps to `size_t`; arithmetic checks `SIZE_MAX`, and division
  and remainder reject zero.
- C containing `usize` has a static assertion that its pointer width matches the
  Rust host that ran IdwC. A mismatched cross-target compile fails explicitly.

## Floating-point semantics

- `f64` targets binary64 C `double` with `FLT_EVAL_METHOD == 0`, nearest-even
  rounding, subnormal support, and no fast-math or finite-math-only mode.
- Floating literals use exact hexadecimal C constants. Volatile temporaries
  round arithmetic results and prevent contraction across operations.
- NaN, infinities, signed zero, division by zero, overflow, and gradual
  underflow follow the documented IEEE-style behavior. NaN payload/sign bits
  are not promised.
- Default `{}` emits the shortest ordinary decimal text that roundtrips to the
  same binary64 value. Fixed precision uses round-half-to-even. Special values
  print `NaN`, `inf`, or `-inf`; signed zero retains its sign.
- Parsing and shortest-output verification require a correctly rounded `strtod`
  implementation. The behavior is tested with macOS libc and in CI targets.

## Rejected language features

The strict subset rejects structs, enums, unions, `char`, string literals as
values, general String/Vec operations, additional integer types, `f32`, casts,
floating remainder, associated constants such as `f64::NAN`, most math methods,
value-producing blocks, labels, closures, arbitrary iterator chains, arbitrary
macros, imports, modules, statics, complete `std`, async, unsafe, raw pointers,
generics, traits, complex ownership/borrowing, and Cargo dependencies in input
programs.

Comments are accepted. Doc comments are attributes and are rejected. Every AST
node is handled by a whitelist; unsupported syntax cannot silently disappear.

## Translation diagnostics

- Library failures use one of three stable `TranspileErrorKind` values:
  `Parse`, `Unsupported`, or `Semantic`.
- `TranspileError::message()` returns the diagnostic without its category or
  position prefix. `TranspileError::location()` returns an optional one-based
  line and column.
- AST validation and semantic failures report an associated syntax location
  when one can be derived from a `syn` span. Whole-program failures without a
  corresponding AST node, such as a missing `main`, may omit it.
- `Display` includes the category, optional position, and message. Parse errors
  preserve the underlying `syn::Error` through `std::error::Error::source()`.

These are translation-time diagnostics. Runtime failure strings and status 101
are separate contracts described in the input, array, and arithmetic sections.

## Compatibility contract

- IdwC 1.x follows Semantic Versioning for the public Rust API and accepted
  strict-mode behavior documented here.
- A minor release may accept additional source programs. It will not silently
  reinterpret a program already accepted by this contract.
- Bug fixes may change generated C while restoring documented behavior.
- Generated whitespace, comments, helper ordering, and internal identifiers
  are implementation details. Consumers must compile the C output rather than
  depend on byte-for-byte source stability.
- Runtime diagnostic text explicitly listed in this document remains part of
  the strict contract. Other explanatory wording may improve without a major
  version change.

## Generated C and verification

The pipeline is:

```text
Rust source → syn AST → validation + semantic analysis → typed IR → C17 source
```

Generated C is a single file. Helper fragments are embedded only when needed.
Function prototypes precede definitions, generated identifiers avoid C keyword
collisions, and temporaries preserve evaluation order. The CLI completes
translation and writes a same-directory temporary file before atomically
renaming it to the requested output.

Tests compare trusted Rust and C programs under the same stdin, including
stdout bytes, stderr where promised, and exit status. Strict C17 compilation,
Clang/GCC coverage, optimized builds, UndefinedBehaviorSanitizer, edge values,
failure paths, and checked-in example output consistency are part of the test
suite.
