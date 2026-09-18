# IdwC

**I don't write C.**

*Because life's too short to manually manage every pointer.*

---

## Tired of writing C?

Tired of dealing with pointers just to get basic things done?

No `String`? Just `char*` and a prayer?

Want to concatenate two strings without questioning your life choices?

Miss the comfort of Rust's type system, ownership, and modern syntax?

**NO. NO. NO. NO MORE.**

Introducing **IdwC** — *I don't write C.*

The revolutionary, groundbreaking, absolutely unnecessary solution to a problem that could probably be solved by just learning C.

Write Rust. Get C. Pretend everything is fine.

```bash
idwc main.rs -o main.c
gcc main.c -o main
./main
```

Congratulations! You've successfully written a C program without writing C.

*Well, technically.*

---

## Why IdwC?

Because I was assigned a C programming course.

Because I already know Rust.

Because I don't want to write `printf("%d\n", x)` for the thousandth time.

And because spending weeks building a transpiler to avoid writing a few lines of C is clearly the most reasonable engineering decision I've ever made.

**Work smarter, not harder.**

*Or, in this case, work significantly harder to avoid working slightly harder.*

---

## Features

- Write Rust, generate C17.
- No Rust runtime required for the generated program.
- No manually translating your homework.
- No existential crisis over `char*`.
- Absolutely zero promises of supporting the entire Rust language.

IdwC currently supports only a small subset of Rust. The milestones and current
translation capabilities below distinguish completed work from planned features.

### Milestones

The current version is **v0.3.0**.

| Version | Goal | Status |
| --- | --- | --- |
| v0.1.0 | End-to-end Hello World translation | Completed |
| v0.2.0 | Variables, basic types, and expressions | Completed (within the subset below) |
| v0.3.0 | Conditionals and loops | Completed (statement forms within the subset below) |
| v0.4.0 | Functions, basic output, and limited typed stdin | Planned |
| v0.5.0 | f64 types, basic floating-point operations, and input/output | Planned |
| v0.6.0 | Fixed-size arrays, indexing, and bounds checks | Planned |
| v0.7.0 | Line input, string splitting/parsing, and limited math functions | Planned |
| v1.0.0 | Stable Rust subset, tests, and documentation | Planned |

Each milestone must be independently buildable, testable, and verifiable.
Planned features are not currently accepted by the translator.

#### Planned floating-point support

**v0.5.0** introduces `f64` separately from string handling: type annotations,
decimal/scientific literals, the `f64` suffix, bindings, assignment, function
parameters/returns, unary negation, `+` / `-` / `*` / `/`, corresponding compound
assignments, and equality/ordering comparisons. Unsuffixed floating literals
default to `f64`. Mixed `i32` / `f64` arithmetic must not silently use C implicit
conversions; casts and floating-point remainder are outside this first stage.
Typed stdin extends to `f64`, with complete token validation and defined parse
error/range behavior. Output covers `println!` and limited fixed precision
such as `{:.2}`, with explicit default-format and rounding rules.

The C target is `double`, subject to validating its representation and floating
environment. NaN, infinities, signed zero, division by zero, overflow/underflow,
and precision must have defined behavior; integer failure rules do not carry
over automatically. Fast-math modes that break those rules are excluded.
Tests must define absolute/relative tolerances for finite numerical results and
compare promised output formats exactly, including special-value cases.

**v0.7.0** adds `parse::<f64>()` and limited `powi(2)` alongside line input for
BMI-style exercises. Math functions require their own precision and
special-value rules before acceptance. `f32` and additional math functions are
deferred to later work. All floating-point features remain **planned**.

#### Planned stdin support

**v0.4.0** introduces a limited typed-input interface for whitespace-separated
`i32` values, including multiple values on one line, cross-line input, and
repeated reads. Its Rust-facing interface will be chosen before implementation;
this milestone does not require full `std::io`, `String`, or `Vec` support.
It also covers output without a newline and explicit stdout flushing for
interactive prompts. EOF, I/O errors, invalid tokens, out-of-range numbers, and
overlong input must have defined behavior. Integration tests will feed the same
stdin to trusted Rust and C programs and compare results.

**v0.5.0** extends typed input to `f64`, as specified above.

**v0.7.0** builds on v0.5.0 floating-point support and v0.6.0 array/index bounds
checks to support limited patterns using
`String::new()`, `stdin().read_line(&mut buffer)`, `trim()`, `split_whitespace()`,
token collection (including the `Vec<&str>` pattern), and `parse::<i32>()` /
`parse::<f64>()`. It also adds limited `powi(2)` for BMI-style exercises using
the existing floating-point rules. Buffer allocation and cleanup,
read_line append/newline behavior, UTF-8, length limits, token lifetimes,
index bounds, EOF, parse failures, and floating-point differences must be
specified and tested. Accepted input/parse `.unwrap()` patterns must fail in a
controlled way. These goals do not imply general support for `String`, `Vec`,
iterators, generics, or borrowing.

All input stages are **planned**, not supported in the current v0.3.0 release.

### Current release: v0.3.0

Requires Rust 1.88 or newer (edition 2024) to build the transpiler, and Clang or GCC
to compile the generated C17 program.

```bash
cargo run -- examples/hello.rs -o /tmp/idwc-hello.c
clang -std=c17 /tmp/idwc-hello.c -o /tmp/idwc-hello
/tmp/idwc-hello
# Hello, World!
```

The variables example exercises arithmetic, boolean expressions, mutation, and
block shadowing:

```bash
cargo run -- examples/variables.rs -o /tmp/idwc-variables.c
clang -std=c17 /tmp/idwc-variables.c -o /tmp/idwc-variables
/tmp/idwc-variables
# total = 18, ready = true
# inner total = 9
# outer total = 17
```

The control-flow example exercises `if` / `else if` / `else`, `while`, `loop`,
`break`, and `continue`:

```bash
cargo run -- examples/control_flow.rs -o /tmp/idwc-control-flow.c
clang -std=c17 /tmp/idwc-control-flow.c -o /tmp/idwc-control-flow
/tmp/idwc-control-flow
# total = 9
# after loop = 6
```

To use the `idwc` executable directly, run `cargo build` and then
`./target/debug/idwc examples/hello.rs -o /tmp/idwc-hello.c`.
Use `--help` for CLI usage. Input must have a `.rs` extension and output `.c`.
A successful translation replaces the specified output file. Translation errors
leave an existing output file untouched and exit with a nonzero status.

#### Currently supported translation

Example of supported input:

```rust
fn main() {
    let mut total: i32 = 6;
    total += 3 * 4;
    let ready: bool = total >= 18;
    println!("total = {}, ready = {}", total, ready);
}
```

- Exactly one private `fn main()`, without parameters, generics, attributes,
  modifiers, or an explicit return type.
- Initialized `let` and `let mut` bindings, with optional `i32` or `bool`
  annotations. Unsuffixed integers default to `i32`; the `i32` suffix is allowed.
- Integer literals in decimal, hex, octal, and binary, including `i32::MIN`
  written as `-2147483648`. Paths such as `i32::MIN` are not supported yet.
- Arithmetic `+`, `-`, `*`, `/`, `%`, unary `-`, integer comparisons
  `<`, `<=`, `>`, `>=`, and equality `==` / `!=` for either supported type.
- Boolean `!`, `&&`, and `||`, with short-circuit evaluation.
- Plain assignment and arithmetic compound assignment (`+=`, `-=`, `*=`, `/=`,
  `%=`) to mutable bindings. Assignment is a statement, not a value expression.
- Multiple statements, nested statement blocks, same-scope and nested shadowing,
  and optional empty main. Value-returning blocks are not supported.
- Statement-form `if` / `else if` / `else` with pure `bool` conditions and
  unit-valued branches. Only the selected branch is executed.
- Unlabeled statement-form `while` and `loop`, including nested loops.
  A `while` condition must be a pure `bool` expression and is re-evaluated
  before every iteration.
- Unlabeled `break` and `continue` inside a loop, acting on the innermost loop.
  `break` cannot carry a value; `continue` in a `while` loop rechecks its condition.
- Pure `i32` / `bool` expression statements with a semicolon may discard their
  result; arithmetic checks still run.
- `println!` with a string literal and sequential `{}` placeholders for `i32`
  and `bool` expressions; a trailing comma is allowed. Empty-argument
  `println!()` is not supported; use `println!("")`.
- Ordinary and raw Rust strings, basic escapes, and UTF-8 text.
- Literal braces use `{{` and `}}`, matching Rust formatting rules.
- Identifiers are currently ASCII only, including raw identifiers such as
  `r#type`. C keyword collisions are avoided by assigning each binding a unique
  generated name. Unicode identifier normalization is not implemented yet.
- Captured or numbered placeholders, format specifications, named arguments,
  embedded NUL, unsupported types/operators, functions, and other
  top-level items are rejected with an error.
- Comments are allowed; doc comments are attributes and are rejected.

Semantic validation rejects undefined or out-of-scope names, assignments to
immutable bindings, mismatched types, and out-of-range integer literals.
Non-boolean conditions and `break` / `continue` outside loops are also rejected.
All branches and loop bodies are validated, including unreachable ones.
Unsupported syntax is reported as an error rather than silently ignored.

Value-producing control-flow expressions, `break` with a value, loop labels,
`if let`, `while let`, and `match` are not supported. Custom functions and
`return`, integer-range `for`, arrays, indexing, standard input, and additional
integer types remain planned. Floating-point types are not supported yet;
limited integer stdin is targeted for v0.4.0, `f64` and typed floating-point input
for v0.5.0, and line/string input for v0.7.0 as described above.
Async, unsafe code, raw
pointers, generics/traits, closures, iterator chains, arbitrary macros, full
`std`, complex ownership/borrowing, and Cargo dependencies in input programs
are outside the initial supported subset.

Text-only output uses `puts`, which adds the newline. Typed output evaluates
all arguments from left to right before writing any part of the message, then
uses fixed C format strings; input text is never used as a C format string.
Booleans print as `true` / `false`. Embedded NUL is rejected because
C string functions would truncate the output. UTF-8 and control bytes use fixed-width octal
escapes, and question marks are escaped to avoid C17 trigraph processing.
Output comparisons cover normal successful writes to stdout; Rust panic and C
I/O error behavior are not guaranteed to match when stdout fails.

#### Integer semantics

Generated C uses `int32_t` and checked arithmetic, matching Rust with
`-C overflow-checks=yes`. Addition, subtraction, multiplication, and negation
are computed in `int64_t` and range-checked before narrowing to `int32_t`.
Division and remainder reject zero divisors and `INT32_MIN / -1` (or `% -1`)
before performing the C operation. Integer division truncates towards zero.

On a checked arithmetic failure the C program flushes stdout, writes a brief
diagnostic to stderr, and exits with status 101. It does not reproduce Rust panic
text, unwinding, or `panic=abort`. Arithmetic remains checked regardless of the
C optimization level; it does not match Rust release-mode wrapping arithmetic.
Out-of-range literals are translation errors. Rust compile-time lints for
constant overflow or unconditional panics are not replicated; supported
arithmetic expressions are checked at runtime, including discarded results.

### Implementation and verification

```text
Rust source → syn AST → whitelist validation + semantic analysis → typed IR → C17 source
```

`src/lib.rs` exposes `transpile(&str) -> Result<String, TranspileError>`.
`src/validate.rs` validates the file and main signature. `src/semantic.rs` uses
a scope stack and symbol table to resolve bindings, infer/check types, and
reject immutable assignment before building the typed IR in `src/ir.rs`.
`src/format.rs` parses the supported `println!` format. `src/codegen.rs` generates
C from IR, using temporaries to preserve evaluation order and conditional
statements for short-circuit operands; it does not inspect the syn AST.
`while` is lowered to a C `for (;;)` with a condition check at the start of each
iteration, so generated arithmetic temporaries and `continue` preserve Rust
evaluation behavior. `loop` uses a C `for (;;)` without a condition check.
`src/main.rs` handles arguments and file I/O. Input programs and custom macros
are never run during translation. The only direct dependency remains `syn`.

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Tests cover expected C output, unsupported syntax, semantic errors, scopes,
short-circuit behavior, branches, nested loops, `break` / `continue`, condition
re-evaluation, CLI success/error paths,
and compilation of trusted fixtures with both Rust and C17. Runtime comparisons
check output bytes and exit status. End-to-end tests require `rustc` and Clang
or GCC on `PATH` and fail explicitly if no C compiler is available. Arithmetic
failure tests also require the compiler's UndefinedBehaviorSanitizer support:
they compile C with `-O2 -fsanitize=undefined -fno-sanitize-recover=undefined`,
compare failure exit status and stdout against Rust with checked arithmetic,
and check C diagnostics. Rust panic diagnostic text is intentionally not compared.
Control-flow executables have a five-second timeout so regressions cannot leave
the test suite stuck in an infinite loop.

If your Rust code is too complicated, IdwC will politely refuse to translate it.

Probably.

---

## The Philosophy

> Why learn C when you can spend three weeks writing a compiler that lets you avoid it?

This project is not about replacing C.

It's not about outperforming GCC.

It's not about reinventing the future of systems programming.

It's about one simple principle:

**I. Don't. Write. C.**

---

*IdwC — Turning a simple homework assignment into a completely unnecessary compiler project since 2026.*
