# IdwC

**I don't write C.**

[繁體中文](README.zh-TW.md) · [Strict semantics](docs/strict-semantics.md) ·
[Stupid Mode](docs/stupid-mode.md) ·
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
semantic analysis, typed IR, and a C code generator. In the default strict
mode, checked arithmetic, bounds-checked arrays, defined input failures, and
binary64 behavior are real guarantees. Unsupported syntax is rejected instead
of being guessed, ignored, or converted into exciting new categories of
undefined behavior.

The exact contract is documented in [Strict semantics](docs/strict-semantics.md).

## Milestones

The current version is **v1.2.1**.

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
| v1.1.0 | Optional Stupid Mode for relaxed, readable C | Completed |
| v1.1.1 | Detailed CLI help and optional `-o` output path | Completed |
| v1.2.0 | Limited strings, fixed-capacity `Vec`, and collection output | Completed |
| v1.2.1 | Stupid Mode string and Vec cleanup | Completed |
| v1.3.0 | Collection ergonomics and additional collection output | Planned |
| v1.4.0 | Bounded String editing | Planned |
| v1.5.0 | Eager array and Vec `.map` translation | Planned |
| v1.6.0 | Short iterator pipelines and reductions | Planned |
| v1.7.0 | Limited pattern-based control flow | Planned |

The v1.2 line supports string-literal `&str` bindings, bounded owned `String` values,
`idwc::io::read_line() -> String`, and fixed-capacity `Vec<T>` for the supported
scalar types. The Vec subset includes `Vec::new()`, both `vec!` forms, `push`,
indexing, `.len()`, assignment, and local moves. Dedicated `{:?}` output for
`i32`, `usize`, and `bool` Vecs and fixed arrays lowers directly to a C loop;
`f64` collections are rejected until their Rust Debug formatting is matched.
`--string-capacity` and `--vec-capacity` configure the generated stack storage.
This does not provide general references, heap allocation, slices, iterators,
or the complete `String`, `Vec`, and `Debug` APIs. Apparently "just use an
array" becomes a design document when Rust semantics are invited to the party.

The planned v1.3.0 collection pass will target `&str` fixed arrays,
Rust-compatible `{:?}` output for `f64` collections, and by-value `for` loops
over supported fixed arrays and Vecs. This directly covers common parallel-array
programs such as a `&str` name array paired with an `f64` score array.

v1.4.0 will then add a small, capacity-checked editing API for bounded String,
starting with `.len()`, `.is_empty()`, `.clear()`, and `.push_str()`.

### Modern Rust lowering roadmap

v1.5.0 is reserved for eager `.map` translation. Fixed arrays will accept the
real Rust form `values.map(|value| expression)`. Vecs will use a real iterator
form such as `values.into_iter().map(|value| expression).collect()`, rather than
an IdwC-only `Vec::map` invention. Both forms lower directly to an indexed C
`for` loop and a fixed-size or fixed-capacity destination. Strict mode retains
checked operations inside the closure body; Stupid Mode emits the readable
operators directly. Initial closures are expression-only and may capture
already-supported scalar bindings.

v1.6.0 will build short, statically understood pipelines on that machinery.
The targets are limited `.filter`, `.enumerate`, `.zip`, `.fold`, `.sum`,
`.any`, and `.all` forms over supported arrays and Vecs. IdwC will fuse a safe
pipeline into ordinary loops where practical, with early exit for `.any` and
`.all`; it will not construct a general iterator runtime or heap-allocated
intermediate collections.

v1.7.0 will target the control-flow sugar needed to make those APIs pleasant:
small tuple destructuring plus limited `match`, `if let`, and `while let` over
well-defined scalar and collection results. General patterns, references,
closures as stored values, and the complete `Iterator` API remain outside this
roadmap. Rust gets to look modern; C gets a loop and no vote in the matter.

These entries are plans rather than current syntax. The strict semantic
contract changes only after each feature is implemented and tested.

## Requirements

- Rust 1.88 or newer to build IdwC.
- Clang or GCC to compile generated C17.
- `-lm` on platforms that require it for generated floating-point programs.

## Installation

Install the released CLI from crates.io with its tested dependency lockfile:

```bash
cargo install idwc --locked
```

Cargo downloads and compiles IdwC, then installs the `idwc` executable in
Cargo's binary directory (normally `~/.cargo/bin`).


To build it yourself from the Git repository instead:

```bash
git clone https://github.com/DanielQu9/idwc.git
cd idwc
cargo build --release --locked
./target/release/idwc --version
```

## Quick start

Translate and compile Hello World with the installed CLI:

```bash
idwc examples/hello.rs -o /tmp/idwc-hello.c
clang -std=c17 /tmp/idwc-hello.c -o /tmp/idwc-hello
/tmp/idwc-hello
# Hello, World!
```

From a source checkout, the same translation can be run without installation:

```bash
cargo run -- examples/hello.rs -o /tmp/idwc-hello.c
```

Generate deliberately relaxed, human-readable C with Stupid Mode:

```bash
idwc --stupid examples/stupid_stdin.rs -o /tmp/idwc-stupid-stdin.c
# Short form: idwc -s examples/stupid_stdin.rs -o /tmp/idwc-stupid-stdin.c
```

The command writes a warning to stderr because the result intentionally omits
strict runtime checks. Its v1.2 collection output keeps simple Vec writes
direct, uses temporaries only where expression side effects require them, and
emits `(void)` warning suppressions only for Vec storage or lengths that are not
read later.
See the [Stupid Mode contract](docs/stupid-mode.md).

Use `--help` for usage and `--version` or `-V` for the version. Input files
must end in `.rs`; output files must end in `.c`. The `-o` option is optional:
`idwc path/program.rs` writes `path/program.c`, while `-o` selects another
location. IdwC finishes translation before atomically replacing the output,
so a translation or partial-write failure does not truncate an existing C
file.

Bounded strings default to 4096 payload bytes and fixed-capacity Vec storage
defaults to 2048 elements. Set them per translation with
`--string-capacity <bytes>` and `--vec-capacity <elements>`; each value must be
between 1 and 65536. These settings change generated stack object sizes, so
large values and many simultaneous collections increase stack usage.

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
floating point, fixed arrays, strings, and fixed-capacity Vecs. Every checked-in
`examples/*.c` file is a golden output generated from its matching Rust source
and verified by tests.

## Current subset at a glance

- Exactly one private `fn main()` with no parameters or explicit return type.
- Private helper functions with scalar parameters and scalar or unit returns;
  forward calls and recursion are supported.
- `i32`, restricted `usize`, `f64`, `bool`, one-dimensional fixed arrays,
  bounded strings, and fixed-capacity scalar Vecs.
- Initialized `let` / `let mut`, assignment, shadowing, and lexical blocks.
- Checked arithmetic, comparisons, boolean operators, and short-circuiting.
- Statement-form `if` / `else if` / `else`, `while`, `loop`, and integer range
  `for`, with unlabeled `break` and `continue`.
- `[T; N]`, array literals, repeat initialization, value copies, `.len()`, and
  checked `usize` indexing for scalar element types.
- `Vec<T>` for `T = i32 | usize | f64 | bool`, with `Vec::new()`, `vec![...]`,
  `vec![value; N]`, `push`, `.len()`, checked indexing, assignment, and moves.
- `print!` / `println!`, including empty calls, sequential `{}` placeholders,
  `{:.0}` through `{:.18}` for `f64`, and limited collection `{:?}` output for
  `i32`, `usize`, and `bool` arrays and Vecs.
- `idwc::io::read_i32()`, `read_f64()`, `read_line()`, and `flush_stdout()`.
- Limited `String::new()` → `read_line(...).unwrap()` → `trim()` /
  `split_whitespace()` → `parse::<i32|f64>().unwrap()` input flows.
- `idwc::io::read_line()` as a shorter way to create and fill one bounded
  `String`; it retains the newline and returns an empty String at empty EOF.
- String-literal `&str` bindings plus bounded `String::new()` /
  `String::from(&str)`, assignment, move checking, and `{}` output.
- The `f64` method `powi(2)`.

Notable exclusions include unsupported String/Vec operations, string
concatenation, cloning, slices, structs, enums, casts, `f32`, other integer types,
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

The default public entry point remains strict:

```rust
pub fn transpile(source: &str) -> Result<String, TranspileError>;
```

Call `transpile_with_options` with `TranspileMode::Stupid` to select readable,
relaxed generation from the library. `transpile(source)` always uses strict
mode.

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

## Stupid Mode

`-s` / `--stupid` prefers C that a human can read and edit: source names are
preserved when possible, visible Unicode remains visible in `u8` string
literals, and direct C operators and standard-library calls replace IdwC
runtime helpers. It emits no generated helper functions, structs, overflow or
bounds checks, input validation, or floating-point environment setup. Functions
written in the Rust source are still emitted normally.

The fun name does not make the behavior vague. Signed overflow, division by
zero, invalid input, out-of-bounds indexing, native floating-point formatting,
and C evaluation order are explicitly outside this mode's guarantees. Parsing,
the supported-AST whitelist, name resolution, and type checking still apply.
Strict mode remains the default and is unchanged. The complete tradeoffs are
documented in [Stupid Mode](docs/stupid-mode.md).

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
