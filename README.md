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

The current version is **v0.2.0**.

| Version | Goal | Status |
| --- | --- | --- |
| v0.1.0 | End-to-end Hello World translation | Completed |
| v0.2.0 | Variables, basic types, and expressions | Completed (within the subset below) |
| v0.3.0 | Conditionals and loops | Planned |
| v0.4.0 | Functions and basic input/output | Planned |
| v0.5.0 | Fixed-size arrays, indexing, and bounds checks | Planned |
| v1.0.0 | Stable Rust subset, tests, and documentation | Planned |

Each milestone must be independently buildable, testable, and verifiable.
Planned features are not currently accepted by the translator.

### Current release: v0.2.0

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
  embedded NUL, unsupported types/operators, control flow, functions, and other
  top-level items are rejected with an error.
- Comments are allowed; doc comments are attributes and are rejected.

Semantic validation rejects undefined or out-of-scope names, assignments to
immutable bindings, mismatched types, and out-of-range integer literals.
Unsupported syntax is reported as an error rather than silently ignored.

Control-flow constructs (`if` / `else`, `while`, `loop`, `break`, `continue`),
custom functions and `return`, integer-range `for`, arrays, indexing, standard
input, and additional integer types remain planned. Async, unsafe code, raw
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
`src/main.rs` handles arguments and file I/O. Input programs and custom macros
are never run during translation. The only direct dependency remains `syn`.

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Tests cover expected C output, unsupported syntax, semantic errors, scopes,
short-circuit behavior, CLI success/error paths,
and compilation of trusted fixtures with both Rust and C17. Runtime comparisons
check output bytes and exit status. End-to-end tests require `rustc` and Clang
or GCC on `PATH` and fail explicitly if no C compiler is available. Arithmetic
failure tests also require the compiler's UndefinedBehaviorSanitizer support:
they compile C with `-O2 -fsanitize=undefined -fno-sanitize-recover=undefined`,
compare failure exit status and stdout against Rust with checked arithmetic,
and check C diagnostics. Rust panic diagnostic text is intentionally not compared.

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
