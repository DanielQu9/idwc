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

The current version is **v0.7.0**.

| Version | Goal | Status |
| --- | --- | --- |
| v0.1.0 | End-to-end Hello World translation | Completed |
| v0.2.0 | Variables, basic types, and expressions | Completed (within the subset below) |
| v0.3.0 | Conditionals and loops | Completed (statement forms within the subset below) |
| v0.4.0 | Functions, basic output, and limited typed stdin | Completed (within the subset below) |
| v0.5.0 | f64 types, basic floating-point operations, and input/output | Completed (within the subset below) |
| v0.6.0 | Fixed-size arrays, indexing, and bounds checks | Completed (within the subset below) |
| v0.7.0 | Line input, string splitting/parsing, and limited math functions | Completed (within the subset below) |
| v1.0.0 | Stable Rust subset, tests, and documentation | Planned |
| v1.1.0 | Optional Stupid Mode for relaxed, readable C output | Planned |

Each milestone must be independently buildable, testable, and verifiable.
Planned features are not currently accepted by the translator.

#### Planned Stupid Mode (v1.1.0)

After the strict subset reaches v1.0.0, `-s` / `--stupid` will select an
explicitly relaxed translation profile:

```bash
idwc --stupid input.rs -o output.c
```

Its priorities are readable C, original names where safe, and compact output;
exact Rust/C semantic equivalence comes after those goals. Strict translation
remains the default and the existing `transpile(source)` API always stays
strict. A separate options API will carry the mode into semantic analysis and
code generation instead of putting translation rules in the CLI.

Planned behavior:

- Use direct, idiomatic C expressions and control flow where possible. The mode
  may omit selected helpers for checked arithmetic, division, array bounds,
  full input validation, exact evaluation order, floating environment checks,
  and Rust-compatible float formatting. Every omitted guarantee must be listed
  in the final mode contract.
- Permit documented floating-point precision and presentation differences,
  using ordinary C `double` operations and simple `printf` formats where that
  produces substantially cleaner C. Such output is never described as strictly
  equivalent to Rust.
- Preserve source variable and function names when they are valid, safe C
  identifiers. C keywords, reserved identifiers, shadowed bindings, generated
  helpers, and duplicate names receive deterministic readable suffixes such as
  `value_2`. Necessary temporaries use descriptive names.
- Keep syn parsing, the AST whitelist, scope/name resolution, and basic type
  checking. Unsupported AST, undefined names, and programs that cannot produce
  valid C still fail explicitly; the mode does not become text replacement.
- Add a short generated-file comment and one CLI notice identifying relaxed
  output and its weaker guarantees.

Acceptance tests will keep the complete strict suite unchanged, compile C17
snapshots for readable Stupid Mode output, and document intentional divergence
cases for precision, overflow, bounds, and evaluation order. This mode is
planned for v1.1.0 and is not implemented by the current CLI.

#### Floating-point support and next stages

**v0.5.0** supports `f64` separately from string handling: type annotations,
decimal/scientific literals, the `f64` suffix, bindings, assignment, function
parameters/returns, unary negation, `+` / `-` / `*` / `/`, corresponding compound
assignments, and equality/ordering comparisons. Unsuffixed floating literals
default to `f64`. Mixed `i32` / `f64` arithmetic must not silently use C implicit
conversions; casts and floating-point remainder are outside this first stage.
Typed stdin uses `idwc::io::read_f64()`, with complete token validation and
defined parse error/range behavior. `print!` / `println!` accept default `{}`
and fixed precision `{:.0}` through `{:.18}` for `f64`.

The C target is binary64 `double`, validated at compilation and startup.
NaN, infinities, signed zero, division by zero, and overflow/underflow use the
rules below; integer failure rules do not apply. Fast-math is rejected.
Tests compare promised formats exactly and use explicit absolute/relative
tolerances for finite numerical results, including boundary and special cases.

**v0.7.0** adds `parse::<f64>()` and exactly `powi(2)` alongside line input for
BMI-style exercises. `powi(2)` evaluates its receiver once and lowers to one
binary64 multiplication, retaining the existing rounding, signed-zero, NaN,
infinity, overflow, and underflow rules. Other exponents, `f32`, and additional
math methods remain unsupported.

#### Stdin support and next stages

**v0.4.0** supports `idwc::io::read_i32()` for ASCII-whitespace-separated
`i32` values, including multiple values on one line, cross-line input, and
repeated reads. `print!` and `idwc::io::flush_stdout()` support interactive
prompts. These exact interfaces have native Rust implementations in this crate;
generated C uses only the C standard library. Full `std::io`, `String`, and
`Vec` are outside this stage. See the input contract below for token limits and
failure behavior. Tests feed identical stdin to trusted Rust and C programs.

**v0.5.0** extends typed input to `f64`, as specified above.

**v0.7.0** supports the exact line-input pattern
`std::io::stdin().read_line(&mut buffer).unwrap()` with a mutable local created
by `String::new()`. It also accepts `input.trim().parse::<T>().unwrap()`, plus
`input.split_whitespace().collect()` or
`input.trim().split_whitespace().collect()` when the result is inferred or
annotated as `Vec<&str>`. The collected tokens support
`tokens[index].parse::<T>().unwrap()` for `T = i32` or `f64`, and `.len()`.

Generated C stores each limited String in a 4096-byte stack buffer and each
collection in 256 stack-allocated byte spans, so no heap allocation or cleanup
is needed. `read_line` appends through the newline, returns successfully on EOF
with no bytes, and validates UTF-8. Splitting and trimming use Rust-compatible
Unicode whitespace. Tokens borrow their source buffer; IdwC conservatively
rejects another `read_line` while a collection remains in lexical scope.
Complete limits and controlled failures are documented below. This support does
not imply general String, Vec, iterator, generic, slice, or borrowing support.

#### Fixed-size array support (v0.6.0)

**v0.6.0** supports local one-dimensional `[T; N]` arrays whose element type is
`i32`, `usize`, `f64`, or `bool`. Array literals, `[value; N]`, whole-array copy
and assignment, `.len()`, element reads, element assignment, and arithmetic
compound assignment are accepted. Lengths are compile-time integer literals
from 0 through 4096. Array bindings retain Rust value-copy behavior rather than
decaying to C pointers.

Every element read or write evaluates its `usize` index once and checks it
before accessing C storage. Failure flushes stdout, writes
`idwc: array index out of bounds` to stderr, and exits with status 101. The C
backend uses one unused storage element for `[T; 0]`, while its logical length
remains zero, so every attempted access still fails before touching storage.
The restricted `usize` support includes literals, bindings, function scalar
parameters/returns, checked arithmetic/comparisons, and formatting with `{}`.

Nested arrays, array function parameters/returns, slices, references, iterator
methods, array equality, and indexing temporary array expressions remain
unsupported. Indexing is currently limited to a named local array binding.

### Current release: v0.7.0

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

The functions/input example reads two integers and calls typed helper functions:

```bash
cargo run -- examples/functions_stdin.rs -o /tmp/idwc-functions.c
clang -std=c17 /tmp/idwc-functions.c -o /tmp/idwc-functions
printf '3 4\n' | /tmp/idwc-functions
# Enter two integers: sum = 7, positive = true
# Run the original Rust example with the same input:
printf '3 4\n' | cargo run --example functions_stdin
```

The floating-point example calculates BMI using typed input and `{:.2}`:

```bash
cargo run -- examples/floating_point.rs -o /tmp/idwc-floating.c
clang -std=c17 /tmp/idwc-floating.c -lm -o /tmp/idwc-floating
printf '70 1.75\n' | /tmp/idwc-floating
# Enter weight (kg) and height (m): BMI = 22.86, below 25 = true
printf '70 1.75\n' | cargo run --example floating_point
```

Link floating-point C programs with `-lm` (required on some platforms).
This example uses the earlier typed-token input API; the v0.7.0 line-input form
is demonstrated separately below.

The arrays example exercises value copying, `.len()`, `usize` indexing,
mutation, and bounds-checked element access:

```bash
cargo run -- examples/arrays.rs -o /tmp/idwc-arrays.c
clang -std=c17 /tmp/idwc-arrays.c -o /tmp/idwc-arrays
/tmp/idwc-arrays
# first = 13, last = 16, original first = 12, length = 4
```

The line-input example uses the limited String/token pipeline and `powi(2)`:

```bash
cargo run -- examples/line_input.rs -o /tmp/idwc-line-input.c
clang -std=c17 /tmp/idwc-line-input.c -lm -o /tmp/idwc-line-input
printf '70 1.75\n' | /tmp/idwc-line-input
# Enter weight (kg) and height (m): BMI = 22.86, below 25 = true
printf '70 1.75\n' | cargo run --example line_input
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
- Additional private, nongeneric functions with `i32` / `usize` / `f64` / `bool`
  scalar parameters (including `mut` parameters) and scalar or unit returns. Omitted return
  types mean unit; helpers may explicitly return `()`. Forward calls and
  recursion are supported; calling `main` and indirect calls are rejected.
- `return;`, `return ();`, and typed `return value;`; typed functions also
  accept a final value expression such as `a + b` or another function call.
  A conservative return check requires an explicit guaranteed return, a final
  value expression, or returns in both `if` / `else` branches. A loop alone does
  not establish a guaranteed return. Parameters are copied and each function
  has independent local scopes. Unit bindings and unit parameters are rejected.
- Initialized `let` and `let mut` bindings, with optional `i32`, `usize`, `f64`,
  `bool`, or one-dimensional fixed-array annotations. Unsuffixed integers
  default to `i32`, except where a `usize` context is required; `i32` and
  `usize` suffixes are allowed.
- Integer literals in decimal, hex, octal, and binary, including `i32::MIN`
  written as `-2147483648`. Paths such as `i32::MIN` are not supported yet.
- Arithmetic `+`, `-`, `*`, `/` and comparisons `<`, `<=`, `>`, `>=` for
  matching `i32`, `usize`, or `f64` operands; unary `-` accepts `i32` and `f64`,
  and `%` accepts `i32` and `usize`. Equality `==` / `!=`
  also supports `bool`. Mixed numeric types are rejected.
- Decimal/scientific floating literals defaulting to `f64`, with optional `f64`
  suffix and underscores (including `1f64`). Nondecimal floating literals and
  `f32` are rejected. Infinite literals are translation errors; tiny literals
  round to representable subnormals or zero.
- The `f64` method `powi(2)` only. Its receiver is evaluated once and squared
  with one binary64 multiplication; other exponents and math methods are rejected.
- Boolean `!`, `&&`, and `||`, with short-circuit evaluation.
- Plain assignment and arithmetic compound assignment (`+=`, `-=`, `*=`, `/=`,
  `%=`) to mutable bindings; `%=` is integer-only. Assignment is a statement,
  not a value expression.
- Local `[T; N]` arrays for `T = i32`, `usize`, `f64`, or `bool`, including
  literals, repeat initialization, value copies, whole-array assignment,
  `.len()`, and `usize` element reads/writes. Lengths are literal values no
  greater than 4096. Every access performs a runtime bounds check.
- Limited local `String` and `Vec<&str>` bindings for the exact
  `String::new()` → `std::io::stdin().read_line(&mut input).unwrap()` →
  `trim()` / `split_whitespace().collect()` → indexed
  `parse::<i32|f64>().unwrap()` pipeline. Token collections also support
  `.len()`. String/Vec assignment, function parameters/returns, and general
  methods remain unsupported.
- Multiple statements, nested statement blocks, same-scope and nested shadowing,
  and optional empty main. Value-returning blocks are not supported.
- Statement-form `if` / `else if` / `else` with `bool` conditions and
  unit-valued branches. Only the selected branch is executed.
- Unlabeled statement-form `while` and `loop`, including nested loops.
  A `while` condition must be a `bool` expression and is re-evaluated
  before every iteration.
- Unlabeled `break` and `continue` inside a loop, acting on the innermost loop.
  `break` cannot carry a value; `continue` in a `while` loop rechecks its condition.
- `i32` / `f64` / `bool` expression statements with a semicolon may discard their
  result; arithmetic checks and call side effects still run. Unit calls may
  omit the semicolon at the end of a block.
- `print!` / `println!` with a string literal and sequential `{}` placeholders for
  `i32`, `usize`, `f64`, and `bool`; fixed precision `{:.0}` through `{:.18}` accepts only
  `f64`. A trailing comma is allowed. Empty-argument
  `println!()` is not supported; use `println!("")`.
- The exact qualified calls `idwc::io::read_i32()`, `idwc::io::read_f64()`, and
  `idwc::io::flush_stdout()`; imports and arbitrary standard-library calls are
  not accepted. Input and function calls may appear in expressions, conditions,
  and output arguments, preserving left-to-right and short-circuit evaluation.
- Ordinary and raw Rust strings, basic escapes, and UTF-8 text.
- Literal braces use `{{` and `}}`, matching Rust formatting rules.
- Identifiers are currently ASCII only, including raw identifiers such as
  `r#type`. C keyword collisions are avoided by assigning each binding a unique
  generated name; functions have separate generated names. Unicode identifier
  normalization is not implemented yet.
- Captured or numbered placeholders, other format specifications, named arguments,
  embedded NUL, unsupported types/operators, and non-function
  top-level items are rejected with an error.
- Comments are allowed; doc comments are attributes and are rejected.

Semantic validation rejects undefined or out-of-scope names, assignments to
immutable bindings, mismatched types, and out-of-range integer literals.
Non-boolean conditions and `break` / `continue` outside loops are also rejected.
All branches and loop bodies are validated, including unreachable ones.
Unsupported syntax is reported as an error rather than silently ignored.

Value-producing control-flow expressions, `break` with a value, loop labels,
`if let`, `while let`, and `match` are not supported. Integer-range `for`, nested
arrays, slices, and integer types other than `i32` / restricted `usize` remain
planned. Array parameters/returns and indexing non-binding expressions are
rejected. `f32`, numeric casts, floating remainder, associated constant paths
(such as `f64::NAN`), math methods other than `powi(2)`, and general String/Vec
operations are rejected.
Async, unsafe code, raw
pointers, generics/traits, closures, iterator chains, arbitrary macros, full
`std`, complex ownership/borrowing, and Cargo dependencies in input programs
are outside the initial supported subset.

Text-only `println!` uses `puts`, which adds the newline; `print!` uses `fputs`.
Typed output evaluates
all arguments from left to right before writing any part of the message, then
uses fixed C format strings; input text is never used as a C format string.
Booleans print as `true` / `false`. Embedded NUL is rejected because
C string functions would truncate the output. UTF-8 and control bytes use fixed-width octal
escapes, and question marks are escaped to avoid C17 trigraph processing.
Output comparisons cover normal successful writes to stdout; Rust panic and C
I/O error behavior are not guaranteed to match when stdout fails.

#### Typed input contract

`idwc::io::read_i32() -> i32` skips ASCII space, tab, LF, CR, vertical tab, and
form feed, then reads one token (up to 128 bytes, including a sign). Tokens must
match `[+-]?[0-9]+` and fit `i32`. Leading zeros and signed zero are allowed.
Hex, underscores, suffixes, decimal points, non-ASCII whitespace/digits, and
partial numeric prefixes are rejected. A valid token immediately followed by
EOF succeeds; the next read fails. No full line is required.

`idwc::io::read_f64() -> f64` uses the same whitespace, token-length and EOF
rules. It accepts `[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)([eE][+-]?[0-9]+)?`,
plus exactly `NaN`, `inf`, `+inf`, and `-inf`. Other spellings, hex floats,
underscores, suffixes, and partial parses are rejected. Finite decimal tokens
that round to infinity, or nonzero tokens that round to zero, are range errors.
Representable nonzero subnormals and signed zero are accepted. Explicit special
tokens do not count as range errors. Integer and float reads may alternate.

Failure flushes stdout, writes one diagnostic to stderr, and exits with status
101. Token length is checked first, then the entire token's syntax, then range;
no value is returned on failure. Errors are:

| Condition | Diagnostic |
| --- | --- |
| EOF before the next token | `idwc: unexpected EOF` |
| Read error (including after token bytes) | `idwc: stdin I/O error` |
| Invalid complete token | `idwc: invalid integer` |
| Outside the i32 range | `idwc: integer out of range` |
| Invalid complete floating-point token | `idwc: invalid float` |
| Numeric float overflow or underflow to zero | `idwc: float out of range` |
| More than 128 token bytes | `idwc: input token too long` |
| Explicit stdout flush failed | `idwc: stdout flush error` |

`idwc::io::flush_stdout() -> ()` checks flush success; use it after `print!`
before reading interactive input. On platforms defining `SIGPIPE`, C flush and
failure paths ignore that signal so a closed output pipe permits a controlled
diagnostic. Other output errors retain the limitation
described above. Native Rust examples link this crate through Cargo; no extra
crate or Rust library is needed to compile the generated C.

#### Limited line-input contract

The v0.7.0 line-input path accepts only local buffers created by
`String::new()` and the exact
`std::io::stdin().read_line(&mut input).unwrap()` call. Each generated C buffer
has 4096 bytes of stack storage. A read appends bytes to existing content,
retains the newline when present, and succeeds without changing the buffer when
EOF occurs before any byte. The 4096-byte limit applies to total accumulated
content across repeated reads. Valid UTF-8 is required after every read.

`trim()` and `split_whitespace()` use the Unicode White_Space property used by
Rust, including non-ASCII separators. A collected `Vec<&str>` is represented by
at most 256 byte spans that borrow the source buffer; it does not allocate or
copy token text. No heap cleanup is required. To keep those spans valid without
implementing Rust's full borrow checker, another `read_line` is rejected while
a token collection from that String remains in lexical scope.

`parse::<i32>()` accepts `[+-]?[0-9]+` and checks the complete i32 range.
`parse::<f64>()` accepts the same decimal/scientific grammar and special values
as the typed float reader, but follows Rust string parsing for range results:
overflow becomes signed infinity and nonzero underflow may become signed zero.
This differs intentionally from `idwc::io::read_f64()`, whose input contract
reports those two cases as range errors. Parsing can consume either
`input.trim()` or `tokens[index]`; token indices are evaluated once and checked
before accessing a span.

All line-input runtime failures flush stdout, write one diagnostic to stderr,
and exit with status 101:

| Condition | Diagnostic |
| --- | --- |
| stdin read error | `idwc: stdin I/O error` |
| Invalid UTF-8 | `idwc: invalid UTF-8 input` |
| Accumulated input exceeds 4096 bytes | `idwc: input line buffer too long` |
| More than 256 split tokens | `idwc: too many input tokens` |
| Token index outside the collection | `idwc: token index out of bounds` |
| Invalid integer / integer outside i32 | `idwc: invalid integer` / `idwc: integer out of range` |
| Invalid floating-point token | `idwc: invalid float` |

The accepted `.unwrap()` forms therefore fail in a controlled way without C
undefined behavior. IdwC does not reproduce Rust panic text or unwinding.

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

Restricted `usize` values map to C `size_t`. Addition, subtraction, and
multiplication check against `SIZE_MAX`; division and remainder reject zero.
These failures use the same integer diagnostics and status 101. This target-sized
type is currently exposed mainly for array lengths and indices. Generated C
containing `usize` includes a static assertion that its target width matches the
Rust host which ran IdwC, so a mismatched cross-target compile fails explicitly.

#### Floating-point semantics and formatting

Generated C requires 53-bit binary64 `double`, a compatible bit layout, and
`FLT_EVAL_METHOD == 0`. Startup sets the C numeric locale and a nontrapping
nearest-even floating environment, checks subnormal support, and fails with
`idwc: unsupported floating-point environment` and status 101 if unavailable.
Fast-math and finite-math-only compilation are rejected. Floating literals use
exact C hex constants; volatile temporaries round each arithmetic result and
prevent FMA contraction across operations.

`+`, `-`, `*`, `/`, and unary `-` use IEEE-style binary64 behavior. Nonzero
division by signed zero produces signed infinity; zero divided by zero produces
NaN. Overflow produces infinity; underflow is gradual and may produce signed
zero. These are floating results, not checked-integer failures. NaN compares
unequal to everything (including itself); ordered comparisons involving NaN are
false. Signed zeros compare equal. NaN payloads/sign bits are not promised.

Default `{}` prints the shortest decimal coefficient that roundtrips to the
same binary64 value, in ordinary decimal notation as Rust Display does.
Decimal midpoint ties in shortest formatting choose the larger magnitude;
fixed precision `{:.0}`..`{:.18}` instead rounds the exact value with
round-half-to-even. Special values print `NaN`, `inf`, `-inf` at any precision.
Signed zero prints `-0` by default and, for example, `-0.00` at precision 2.
The C formatter expands binary64 into bounded exact decimal digits, searches
roundtripping candidates for the shortest form, and rounds fixed forms using
integer digits. It does not delegate Rust Display to `%f` or `%.17g`.
Parsing and shortest candidate verification require correctly rounded `strtod`
from the target C library; verified here with macOS Clang/libc.

### Implementation and verification

```text
Rust source → syn AST → whitelist validation + semantic analysis → typed IR → C17 source
```

`src/lib.rs` exposes `transpile(&str) -> Result<String, TranspileError>`.
`src/validate.rs` validates the file and function signatures. `src/semantic.rs` uses
a scope stack and symbol table to resolve bindings, infer/check types, and
reject immutable assignment before building the typed IR in `src/ir.rs`. It
registers function signatures before checking bodies, resolves calls, and checks
parameter/return types and guaranteed returns.
`src/format.rs` parses the supported `print!` / `println!` format. `src/codegen.rs` generates
C from IR, using temporaries to preserve evaluation order and conditional
statements for short-circuit operands; it does not inspect the syn AST.
Array initialization and assignment also use ordered temporaries, preserve
whole-array value copies, and check each index before generating an access.
`while` is lowered to a C `for (;;)` with a condition check at the start of each
iteration, so generated arithmetic temporaries and `continue` preserve Rust
evaluation behavior. `loop` uses a C `for (;;)` without a condition check.
Function prototypes precede definitions; call arguments are saved into ordered
temporaries. `src/io.rs` implements the native Rust typed-input/flush interface,
mirrored by bounded C runtime helpers. `src/runtime/*.c` contains float
environment, exact decimal formatting, shared token-reading helpers, and the
bounded UTF-8 line/token-span runtime, embedded only when needed. String and
token operations are lowered to dedicated IR nodes rather than general-purpose
ownership, iterator, or generic machinery. The generated C remains a single
standalone file.
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
re-evaluation, functions/recursion/returns, argument side effects, stdin token
validation and boundaries, EOF and I/O errors, prompts flushed before input,
explicit flush failure with a closed output pipe,
floating-point type errors, arithmetic, special values, rounding midpoints,
subnormal/large-value formatting, and float token range validation,
fixed-array initialization/copy/mutation, `usize`, index evaluation order, and
bounds failures under UndefinedBehaviorSanitizer,
line append/EOF behavior, Unicode whitespace, invalid UTF-8, buffer/token
limits, token bounds and lifetime rejection, i32/f64 string parsing, and
`powi(2)` including special/range values,
CLI success/error paths,
and compilation of trusted fixtures with both Rust and C17. Runtime comparisons
check output bytes and exit status. End-to-end tests require `rustc` and Clang
or GCC on `PATH` and fail explicitly if no C compiler is available. Arithmetic
failure tests also require the compiler's UndefinedBehaviorSanitizer support:
they compile C with `-O2 -fsanitize=undefined -fno-sanitize-recover=undefined`,
compare failure exit status and stdout against Rust with checked arithmetic,
and check C diagnostics. Rust panic diagnostic text is intentionally not compared.
Function and input fixtures also run optimized C with UBSan and compare stdout,
stderr, and exit status against Rust. The interactive prompt test observes the
prompt before sending stdin. Float comparisons include fixed seeded bit patterns
and neighbors of powers of two, using exact output checks plus finite numerical
tolerances (`1e-15` absolute + `1e-14` relative). Float fixtures link `-lm`;
fast-math rejection is tested explicitly. The test runner drains output pipes
while programs run, including large decimal expansions. Control-flow and input executables have a
five-second timeout so regressions cannot leave
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
