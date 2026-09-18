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

**Disclaimer:** IdwC currently supports only a small subset of Rust. Features mentioned above describe project goals and may not be implemented yet.

### v0.1.0: working Hello World

Requires Rust with edition 2024 support to build the transpiler, and Clang or GCC
to compile the generated C17 program.

```bash
cargo run -- examples/hello.rs -o /tmp/idwc-hello.c
clang -std=c17 /tmp/idwc-hello.c -o /tmp/idwc-hello
/tmp/idwc-hello
# Hello, World!
```

To use the `idwc` executable directly, run `cargo build` and then
`./target/debug/idwc examples/hello.rs -o /tmp/idwc-hello.c`.
Use `--help` for CLI usage. Input must have a `.rs` extension and output `.c`.
A successful translation replaces the specified output file. Translation errors
leave an existing output file untouched and exit with a nonzero status.

Supported input:

```rust
fn main() {
    println!("Hello, World!");
}
```

- Exactly one private `fn main()`, without parameters, generics, attributes,
  modifiers, or an explicit return type.
- Exactly one `println!` with one string literal; a trailing comma is allowed.
- Ordinary and raw Rust strings, basic escapes, and UTF-8 text.
- Literal braces use `{{` and `}}`, matching Rust formatting rules.
- Format arguments, placeholders (including captured variables), embedded NUL,
  other statements, and other top-level items are rejected with an error.
- Comments are allowed; doc comments are attributes and are rejected.

The MVP uses `puts`, which adds the newline. Embedded NUL is rejected because
`puts` would truncate the output. UTF-8 and control bytes use fixed-width octal
escapes, and question marks are escaped to avoid C17 trigraph processing.
Output comparisons cover normal successful writes to stdout; Rust panic and C
I/O error behavior are not guaranteed to match when stdout fails.

### Implementation and verification

```text
Rust source → syn AST → whitelist validation → Program IR → C17 source
```

`src/lib.rs` exposes `transpile(&str) -> Result<String, TranspileError>`.
`src/validate.rs` validates the AST and decodes literal format braces into the
small IR in `src/ir.rs`. `src/codegen.rs` generates C from that validated IR;
it does not inspect the syn AST. `src/main.rs` handles arguments and file I/O.
There is no separate symbol table or type checker yet because the MVP accepts
no variables or expressions. Input programs and custom macros are never run
during translation.

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Tests cover expected C output, unsupported syntax, CLI success/error paths,
and compilation of trusted fixtures with both Rust and C17. Runtime comparisons
check output bytes and exit status. End-to-end tests require `rustc` and Clang
or GCC on `PATH` and fail explicitly if no C compiler is available.

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
