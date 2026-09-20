# IdwC

**I don't write C.**

[繁體中文](README.zh-TW.md) · [Strict semantics](docs/strict-semantics.md) ·
[Changelog](CHANGELOG.md) · [Release process](RELEASING.md)

*Because life's too short to manually manage every pointer.*

---

## Tired of writing C?

Tired of dealing with pointers just to get basic things done?

No `String`? Just `char*` and a prayer?

Want to concatenate two strings without questioning your life choices?

Miss the comfort of Rust's type system, ownership, and modern syntax?

**NO. NO. NO. NO MORE.**

Introducing **IdwC** — *I don't write C.*

The revolutionary, groundbreaking, absolutely unnecessary solution to a
problem that could probably be solved by just learning C.

Write Rust. Get C. Pretend everything is fine.

```bash
idwc main.rs -o main.c
gcc -std=c17 main.c -o main
./main
```

Congratulations! You've successfully written a C program without writing C.

*Well, technically.*

---

## Why IdwC?

Because I was assigned a C programming course.

Because I already know Rust.

Because I don't want to write `printf("%d\n", x)` for the thousandth time.

And because spending weeks building a transpiler to avoid writing a few lines
of C is clearly the most reasonable engineering decision I've ever made.

**Work smarter, not harder.**

*Or, in this case, work significantly harder to avoid working slightly harder.*

---

## Features

- Write Rust, generate C17.
- No Rust runtime required for the generated program.
- No manually translating your homework.
- No existential crisis over `char*`.
- Absolutely zero promises of supporting the entire Rust language.

IdwC does not support the complete Rust language. In particular, its limited
line-input `String` is not a general growable Rust String and does not support
concatenation yet. Apparently replacing `strcat` with an ownership model takes
more than one dramatic README introduction.

What it does support is handled by a real `syn` AST, whitelist validation,
semantic analysis, typed IR, and a C code generator. Checked arithmetic,
bounds-checked arrays, defined input failures, and binary64 behavior are real
guarantees. Unsupported syntax is rejected instead of being guessed, ignored,
or converted into exciting new categories of undefined behavior.

The exact contract is documented in [Strict semantics](docs/strict-semantics.md).

## Milestones

The current version is **v1.0.0**.

| Version | Goal | Status |
| --- | --- | --- |
| v0.1.0 | End-to-end Hello World translation | Completed |
| v0.2.0 | Variables, basic types, and expressions | Completed |
| v0.3.0 | Conditionals and loops | Completed |
| v0.4.0 | Functions, output, and limited typed stdin | Completed |
| v0.5.0 | Strict `f64` arithmetic and input/output | Completed |
| v0.6.0 | Fixed arrays, indexing, and bounds checks | Completed |
| v0.7.0 | Line input, token parsing, and `powi(2)` | Completed |
| v0.8.0 | Integer range `for` and core-subset completion | Completed |
| v0.9.0 | API, diagnostics, portability, and release hardening | Completed |
| v1.0.0 | Frozen strict Rust subset and stable documentation | Completed |
| v1.1.0 | Optional Stupid Mode for relaxed, readable C | Planned |

Additional integer types and general String operations are deferred. The v1.0
release stabilizes the existing subset rather than expanding toward full Rust.

## Requirements

- Rust 1.88 or newer to build IdwC.
- Clang or GCC to compile generated C17.
- `-lm` on platforms that require it for generated floating-point programs.

## Quick start

Install the released CLI from crates.io:

```bash
cargo install idwc
```

Build and translate Hello World:

```bash
cargo build
./target/debug/idwc examples/hello.rs -o /tmp/idwc-hello.c
clang -std=c17 /tmp/idwc-hello.c -o /tmp/idwc-hello
/tmp/idwc-hello
# Hello, World!
```

The same command can be run through Cargo:

```bash
cargo run -- examples/hello.rs -o /tmp/idwc-hello.c
```

Use `--help` for usage and `--version` or `-V` for the version. Input files
must end in `.rs`; output files must end in `.c`. IdwC finishes translation
before atomically replacing the output, so a translation or partial-write
failure does not truncate an existing C file.

### Integer range example

```rust
fn main() {
    let mut total = 0;
    for value in 1..5 {
        if value == 2 {
            continue;
        }
        total += value;
    }
    println!("total = {}", total);
}
```

Translate and run the checked-in example:

```bash
cargo run -- examples/ranges.rs -o /tmp/idwc-ranges.c
clang -std=c17 /tmp/idwc-ranges.c -o /tmp/idwc-ranges
/tmp/idwc-ranges
```

`for binding in start..end` and `start..=end` accept matching `i32` or `usize`
bounds. Bounds are evaluated once from left to right. The iteration binding has
its own scope, `break` and `continue` work normally, and inclusive ranges stop
safely at the maximum integer value.

### Line-input example

```bash
cargo run -- examples/line_input.rs -o /tmp/idwc-line-input.c
clang -std=c17 /tmp/idwc-line-input.c -lm -o /tmp/idwc-line-input
printf '70 1.75\n' | /tmp/idwc-line-input
# Enter weight (kg) and height (m): BMI = 22.86, below 25 = true
```

Other examples cover variables, control flow, functions and typed stdin,
floating point, and fixed arrays. Every checked-in `examples/*.c` file is a
golden output generated from its matching Rust source and verified by tests.

## Current subset at a glance

- Exactly one private `fn main()` with no parameters or explicit return type.
- Private helper functions with scalar parameters and scalar or unit returns;
  forward calls and recursion are supported.
- `i32`, restricted `usize`, `f64`, `bool`, and one-dimensional fixed arrays.
- Initialized `let` / `let mut`, assignment, shadowing, and lexical blocks.
- Checked arithmetic, comparisons, boolean operators, and short-circuiting.
- Statement-form `if` / `else if` / `else`, `while`, `loop`, and integer range
  `for`, with unlabeled `break` and `continue`.
- `[T; N]`, array literals, repeat initialization, value copies, `.len()`, and
  checked `usize` indexing for scalar element types.
- `print!` / `println!`, including empty calls, sequential `{}` placeholders,
  and `{:.0}` through `{:.18}` for `f64`.
- `idwc::io::read_i32()`, `read_f64()`, and `flush_stdout()`.
- Limited `String::new()` → `read_line(...).unwrap()` → `trim()` /
  `split_whitespace()` → `parse::<i32|f64>().unwrap()` input flows.
- The `f64` method `powi(2)`.

Notable exclusions include general String/Vec operations, string
concatenation, slices, structs, enums, casts, `f32`, other integer types,
general iterators, closures, `match`, async, unsafe, pointers, generics, traits,
arbitrary macros, complete `std`, and input-program Cargo dependencies.

See [Strict semantics](docs/strict-semantics.md) for exact syntax, limits,
evaluation order, diagnostics, and C runtime behavior.

## Architecture

```text
Rust source
    → syn AST
    → whitelist validation
    → semantic analysis and scope/type checks
    → typed custom IR
    → C17 code generation
```

The public entry point is:

```rust
pub fn transpile(source: &str) -> Result<String, TranspileError>;
```

Translation failures expose a stable `TranspileErrorKind` (`Parse`,
`Unsupported`, or `Semantic`), the unprefixed message, and an optional
one-based `SourceLocation`. `Display` includes the category and location for
CLI-friendly diagnostics. Errors concerning the whole program, such as a
missing `main`, may not have a source location.

IdwC 1.x follows Semantic Versioning for this public Rust API and the accepted
strict-mode behavior described in the semantic contract. Generated C remains
readable, but its whitespace, helper layout, and internal identifier names are
not a stable text-level API.

`src/main.rs` handles only CLI arguments and file I/O. Translation rules stay
in the library. Runtime C fragments under `src/runtime/` are embedded only when
the translated program needs them. The generated result remains one C file.

## Development and verification

```bash
cargo fmt -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

End-to-end tests compile trusted Rust and generated C programs, feed them the
same input, and compare output bytes and exit status. Checked arithmetic and
bounds failures are also compiled with UndefinedBehaviorSanitizer. CI runs the
suite at the declared Rust 1.88 MSRV with Clang and GCC on Linux and Clang on
macOS.

## Planned Stupid Mode

With strict mode stable in v1.0.0, v1.1.0 will add `-s` / `--stupid`. It will
prefer clean, editable C and original source names where safe, and may relax
documented checks or floating-point guarantees. Strict mode will remain the
default, and `transpile(source)` will always retain strict behavior.

Stupid Mode will still parse with `syn`, validate supported AST, resolve names,
and reject programs that cannot produce valid C. It will not become string
replacement.

## License

IdwC is available under the [MIT License](LICENSE).

If your Rust code is too complicated, IdwC will politely refuse to translate it.

Probably.

---

## The Philosophy

> Why learn C when you can spend three weeks writing a compiler that lets you
> avoid it?

This project is not about replacing C.

It's not about outperforming GCC.

It's not about reinventing the future of systems programming.

It's about one simple principle:

**I. Don't. Write. C.**

---

*IdwC — Turning a simple homework assignment into a completely unnecessary
compiler project since 2026.*
