use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// 提供測試專用目錄，避免測試互相覆寫或留下編譯產物。
struct TestDir(PathBuf);

impl TestDir {
    /// 使用 PID 與序號配置獨立目錄，若遇既有目錄則嘗試下一個。
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "idwc-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("無法建立測試目錄：{error}"),
            }
        }
    }

    /// 取得此測試目錄內的檔案路徑。
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        // 僅清理本測試透過 create_dir 新建且持有的目錄。
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 執行可信任測試命令，失敗時附上編譯器或 CLI 診斷。
fn successful(command: &mut Command) -> Output {
    let output = command.output().expect("需要可執行的 rustc 與 Clang/GCC");
    assert!(
        output.status.success(),
        "命令 {command:?} 失敗：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// 限制可信任迴圈案例的執行時間，避免條件重算或 continue 迴歸造成測試卡住。
fn run_with_timeout(command: &mut Command) -> Output {
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("無法啟動測試程式");
    wait_with_timeout(child)
}

fn wait_with_timeout(mut child: Child) -> Output {
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            let _ = child.wait_with_output();
            panic!("測試程式執行逾時");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run_with_input(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    wait_with_timeout(child)
}

/// Rust 參考程式引用相同的公開 I/O 實作；轉譯器只解析原始的受限程式。
fn compile_pair(dir: &TestDir, source: &str) -> (PathBuf, PathBuf) {
    let rust_source = dir.file("input.rs");
    let c_source = dir.file("output.c");
    let rust_binary = dir.file("rust-output");
    let c_binary = dir.file("c-output");
    let io_source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/io.rs");
    fs::write(
        &rust_source,
        format!("mod idwc {{ #[path = {io_source:?}] pub mod io; }}\n{source}"),
    )
    .unwrap();
    fs::write(&c_source, idwc::transpile(source).unwrap()).unwrap();
    successful(
        Command::new("rustc")
            .args(["--edition=2024", "-C", "overflow-checks=yes"])
            .arg(&rust_source)
            .arg("-o")
            .arg(&rust_binary),
    );
    successful(
        Command::new(c_compiler())
            .args([
                "-std=c17",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic-errors",
                "-O2",
                "-fsanitize=undefined",
                "-fno-sanitize-recover=undefined",
            ])
            .arg(&c_source)
            .arg("-o")
            .arg(&c_binary),
    );
    (rust_binary, c_binary)
}

fn assert_same_output(rust: &Output, c: &Output) {
    assert_eq!(rust.status.code(), c.status.code());
    assert_eq!(rust.stdout, c.stdout);
    assert_eq!(rust.stderr, c.stderr);
}

/// 優先使用 Clang，未安裝時改用 GCC；兩者都缺少時明確失敗。
fn c_compiler() -> &'static str {
    for compiler in ["clang", "gcc"] {
        if Command::new(compiler)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return compiler;
        }
    }
    panic!("端到端測試需要 Clang 或 GCC");
}

/// 以嚴格 C17 編譯測試產生的 C 原始碼。
fn compile_c(source: &Path, executable: &Path) {
    successful(
        Command::new(c_compiler())
            .args([
                "-std=c17",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic-errors",
            ])
            .arg(source)
            .arg("-o")
            .arg(executable),
    );
}

/// 比較可信任 Rust 與 C 案例的輸出位元組、stderr 與退出狀態。
#[test]
fn generated_c_matches_rust_output() {
    let cases = [
        include_str!("../examples/hello.rs"),
        r#"fn main() { println!(""); }"#,
        r#"fn main() { println!("quote: \" slash: \\ newline:\n tab:\t CR:\r"); }"#,
        r#"fn main() { println!("control:\x01F\x078\x089\x0C0\x7F"); }"#,
        r#"fn main() { println!("中文 🦀 café"); }"#,
        r##"fn main() { println!(r#"raw: "quoted" \n"#); }"##,
        r#"fn main() { println!("{{}} {{{{text}}}} 100% %s %n"); }"#,
        r#"fn main() { println!("??/ ??= ??' ??( ??) ??! ??< ??> ??-"); }"#,
        r#"fn main() { println!("line\
            continuation"); }"#,
        include_str!("../examples/variables.rs"),
        r#"fn main() {
            let mut x: i32 = 7;
            x += 3; x -= 2; x *= 4; x /= 3; x %= 7;
            println!("{}", x);
            x = 99;
            println!("{}", x);
        }"#,
        r#"fn main() {
            let x = 1;
            let x = x + 1;
            { let x = x * 3; println!("inner {}", x); }
            println!("outer {}", x);
            let x = true;
            println!("shadow {}", x);
        }"#,
        r#"fn main() {
            let mut x = 1;
            { x = 2; { let mut x = 3; x += 4; println!("{}", x); } }
            println!("{}", x);
        }"#,
        r#"fn main() {
            let x = 2 + 3 * 4;
            let y = (2 + 3) * 4;
            println!("{} {} {}", x, y, -(x - y));
        }"#,
        r#"fn main() {
            let x = -7;
            println!("{} {} {} {}", x / 3, x % 3, 7 / -3, 7 % -3);
        }"#,
        r#"fn main() {
            let min: i32 = -(2147483648);
            let max = 2_147_483_647i32;
            println!("{} {} {}", min, max, min + max);
        }"#,
        r#"fn main() {
            let a = 3;
            let b = 4;
            let t = true;
            let f = false;
            println!("{} {} {} {} {} {} {} {}", a < b, a <= b, a > b, a >= b,
                a == b, a != b, t == f, !f);
            println!("{} {}", t && (a < b), f || (a == 3));
        }"#,
        r#"fn main() {
            let zero = 0;
            let max = 2147483647;
            println!("{} {}", false && (1 / zero == 0), true || (max + 1 == 0));
            println!("{}", (true || (1 / zero == 0)) && (false || true));
        }"#,
        r#"fn main() {
            let int = 1;
            let printf = 2;
            let r#return = 3;
            let idwc_v0 = 4;
            let idwc_t0 = 5;
            let r#type = false;
            println!("{} {} {} {} {} {}", int, printf, r#return, idwc_v0, idwc_t0, r#type);
        }"#,
        r#"fn main() {
            let r#value = 12;
            println!("{{{}}} {} 100% %n 中文", value, !true);
        }"#,
        "fn main() {}",
        "fn main() { let _unused = 42; { let _unused = true; } }",
        "fn main() { let mut _unused = 1; _unused = 2; _unused += 3; }",
        r#"fn main() { let x = 2; (x + 1) * 3; println!("ok"); }"#,
    ];
    let dir = TestDir::new();
    let rust_source = dir.file("input.rs");
    let c_source = dir.file("output.c");
    let rust_binary = dir.file("rust-output");
    let c_binary = dir.file("c-output");
    for source in cases {
        fs::write(&rust_source, source).unwrap();
        fs::write(&c_source, idwc::transpile(source).unwrap()).unwrap();
        successful(
            Command::new("rustc")
                .arg("--edition=2024")
                .arg(&rust_source)
                .arg("-o")
                .arg(&rust_binary),
        );
        compile_c(&c_source, &c_binary);
        let rust_output = successful(&mut Command::new(&rust_binary));
        let c_output = successful(&mut Command::new(&c_binary));
        assert_eq!(rust_output.status.code(), c_output.status.code());
        assert_eq!(rust_output.stdout, c_output.stdout, "案例：{source}");
        assert_eq!(rust_output.stderr, c_output.stderr, "案例：{source}");
    }
}

/// 比較控制流程的輸出、scope、短路條件與巢狀 break／continue。
#[test]
fn control_flow_matches_rust_output() {
    let cases = [
        include_str!("../examples/control_flow.rs"),
        r#"fn main() {
            if true { println!("yes"); } else { println!("no"); }
            if false { println!("no"); }
            if false { println!("no"); } else { println!("yes"); }
        }"#,
        r#"fn main() {
            let x = 3;
            if x == 1 { println!("one"); } else if x == 2 { println!("two"); }
            else if x == 3 { println!("three"); } else { println!("other"); }
            if x == 1 {} else if x == 2 {} else { println!("fallback"); }
        }"#,
        r#"fn main() {
            let zero = 0;
            if true { println!("selected"); }
            else if 1 / zero == 0 { println!("bad"); }
            if false { let x = 1 / zero; } else { println!("safe"); }
            while false { let x = 1 / zero; }
        }"#,
        r#"fn main() {
            let mut x = 1;
            if true { let x = x + 1; println!("{}", x); }
            else { let x = false; println!("{}", x); }
            if false {} else { x += 2; }
            println!("{}", x);
        }"#,
        r#"fn main() {
            let mut n = 0;
            while n * 2 < 10 { n += 1; continue; }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut n = 0;
            while n < 3 && 6 / (3 - n) > 0 { n += 1; continue; }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut active = true;
            let mut n = 0;
            while active { n += 1; active = false; }
            while false { n += 100; }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut n = 0;
            let mut sum = 0;
            while n < 10 {
                n += 1;
                if n % 2 == 0 { continue; }
                if n == 7 { break; }
                sum += n;
            }
            println!("{} {}", n, sum);
        }"#,
        r#"fn main() {
            let mut n = 0;
            loop { n += 1; if n < 3 { continue; } break; }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut n = 0;
            while n < 5 { n += 1; { if n == 2 { break; } } }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut outer = 0;
            let mut count = 0;
            while outer < 3 {
                outer += 1;
                let mut inner = 0;
                loop {
                    inner += 1;
                    if inner == 1 { continue; }
                    count += 1;
                    if inner == 3 { break; }
                }
                count += 10;
            }
            println!("{} {}", outer, count);
        }"#,
        r#"fn main() {
            let mut n = 0;
            loop {
                n += 1;
                while true { break; }
                if n == 2 { break; }
                continue;
            }
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut n = 0;
            while n < 3 {
                let n = n + 10;
                println!("{}", n);
                break;
            }
            n = 3;
            println!("{}", n);
        }"#,
        r#"fn main() {
            let mut n = 0;
            let mut sum = 0;
            while n < 3 {
                let x = n + 1;
                sum += x;
                n += 1;
            }
            println!("{}", sum);
        }"#,
        r#"fn main() {
            let zero = 0;
            if false && (1 / zero == 0) { println!("bad"); }
            else if true || (1 / zero == 0) { println!("safe"); }
            while false && (1 / zero == 0) { println!("bad"); }
        }"#,
        r##"fn main() { if true { println!("#![cfg(any())] # [] {{}}"); } }"##,
        r#"fn main() { loop { break } println!("tail break"); }"#,
    ];
    let dir = TestDir::new();
    let rust_source = dir.file("input.rs");
    let c_source = dir.file("output.c");
    let rust_binary = dir.file("rust-output");
    let c_binary = dir.file("c-output");
    for source in cases {
        fs::write(&rust_source, source).unwrap();
        fs::write(&c_source, idwc::transpile(source).unwrap()).unwrap();
        successful(
            Command::new("rustc")
                .args([
                    "--edition=2024",
                    "-A",
                    "unconditional_panic",
                    "-C",
                    "overflow-checks=yes",
                ])
                .arg(&rust_source)
                .arg("-o")
                .arg(&rust_binary),
        );
        compile_c(&c_source, &c_binary);
        let rust_output = run_with_timeout(&mut Command::new(&rust_binary));
        let c_output = run_with_timeout(&mut Command::new(&c_binary));
        assert!(rust_output.status.success(), "Rust 案例：{source}");
        assert_eq!(
            rust_output.status.code(),
            c_output.status.code(),
            "案例：{source}"
        );
        assert_eq!(rust_output.stdout, c_output.stdout, "案例：{source}");
        assert_eq!(rust_output.stderr, c_output.stderr, "案例：{source}");
    }
}

/// 檢查算術失敗、求值順序與輸出時機，並用 UBSan 確認 C 沒有 UB。
#[test]
fn arithmetic_failures_are_checked_without_undefined_behavior() {
    let cases = [
        ("let x = 2147483647; let y = x + 1;", "integer overflow"),
        ("let x = -2147483648; let y = x - 1;", "integer overflow"),
        ("let x = 2147483647; let y = x * 2;", "integer overflow"),
        ("let x = -2147483648; let y = -x;", "integer overflow"),
        ("let x = -2147483648; let y = x / -1;", "integer overflow"),
        ("let x = -2147483648; let y = x % -1;", "integer overflow"),
        ("let zero = 0; let y = 1 / zero;", "division by zero"),
        ("let zero = 0; let y = 1 % zero;", "division by zero"),
        ("let mut x = 2147483647; x += 1;", "integer overflow"),
        ("let x = 2147483647; x + 1;", "integer overflow"),
        // 左 operand 與第一個 println! 引數應先失敗，且不先輸出 prefix。
        (
            "let x = 2147483647; let zero = 0; let y = (x + 1) + (1 / zero);",
            "integer overflow",
        ),
        (
            r#"let x = 2147483647; let zero = 0; println!("prefix {} {}", x + 1, 1 / zero);"#,
            "integer overflow",
        ),
        (
            r#"let x = 2147483647; let zero = 0; println!("prefix {} {}", 1 / zero, x + 1);"#,
            "division by zero",
        ),
        // 非短路的右 operand 必須確實求值。
        (
            "let zero = 0; let y = true && (1 / zero == 0);",
            "division by zero",
        ),
        (
            "let zero = 0; let y = false || (1 / zero == 0);",
            "division by zero",
        ),
        ("let x = 2147483647; if x + 1 == 0 {}", "integer overflow"),
        (
            "let zero = 0; while 1 / zero > 0 { break; }",
            "division by zero",
        ),
        (
            "let mut n = 0; while 1 / (1 - n) > 0 { n += 1; continue; }",
            "division by zero",
        ),
        (
            "loop { let x = 2147483647; x + 1; break; }",
            "integer overflow",
        ),
    ];
    let dir = TestDir::new();
    let rust_source = dir.file("input.rs");
    let c_source = dir.file("output.c");
    let rust_binary = dir.file("rust-output");
    let c_binary = dir.file("c-output");
    for (body, diagnostic) in cases {
        let source = format!("fn main() {{ println!(\"before\"); {body} println!(\"after\"); }}");
        fs::write(&rust_source, &source).unwrap();
        fs::write(&c_source, idwc::transpile(&source).unwrap()).unwrap();
        // 關閉 rustc 的常數運算 lint，才能執行可信任的失敗案例。
        successful(
            Command::new("rustc")
                .args([
                    "--edition=2024",
                    "-A",
                    "arithmetic_overflow",
                    "-A",
                    "unconditional_panic",
                    "-C",
                    "overflow-checks=yes",
                ])
                .arg(&rust_source)
                .arg("-o")
                .arg(&rust_binary),
        );
        successful(
            Command::new(c_compiler())
                .args([
                    "-std=c17",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pedantic-errors",
                    "-O2",
                    "-fsanitize=undefined",
                    "-fno-sanitize-recover=undefined",
                ])
                .arg(&c_source)
                .arg("-o")
                .arg(&c_binary),
        );
        let rust_output = run_with_timeout(&mut Command::new(&rust_binary));
        let c_output = run_with_timeout(&mut Command::new(&c_binary));
        assert_eq!(rust_output.status.code(), Some(101), "Rust 案例：{source}");
        assert_eq!(c_output.status.code(), Some(101), "C 案例：{source}");
        assert_eq!(rust_output.stdout, b"before\n", "Rust 案例：{source}");
        assert_eq!(c_output.stdout, rust_output.stdout, "C 案例：{source}");
        assert_eq!(
            String::from_utf8_lossy(&c_output.stderr),
            format!("idwc: {diagnostic}\n"),
            "C 案例：{source}"
        );
    }
}

/// 從 CLI 讀檔轉譯，再編譯與執行 C，驗證完整使用流程。
#[test]
fn cli_transpiles_file_to_runnable_c() {
    let dir = TestDir::new();
    let input = dir.file("hello world.rs");
    let output = dir.file("hello world.c");
    let executable = dir.file("hello");
    fs::write(&input, include_str!("../examples/hello.rs")).unwrap();
    let result = successful(
        Command::new(env!("CARGO_BIN_EXE_idwc"))
            .arg(&input)
            .arg("-o")
            .arg(&output),
    );
    assert!(result.stdout.is_empty());
    compile_c(&output, &executable);
    assert_eq!(
        successful(&mut Command::new(executable)).stdout,
        b"Hello, World!\n"
    );
}

/// 錯誤輸入不應建立或覆寫輸出檔案，且應提供非零退出狀態。
#[test]
fn cli_reports_errors_without_overwriting_output() {
    let dir = TestDir::new();
    let input = dir.file("invalid.rs");
    let output = dir.file("output.c");
    fs::write(&input, "fn main() { let x = 1.0; }").unwrap();
    fs::write(&output, "existing output").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("不支援的語法"));
    assert_eq!(fs::read_to_string(&output).unwrap(), "existing output");

    let missing = dir.file("missing.rs");
    let new_output = dir.file("new.c");
    let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
        .arg(missing)
        .arg("-o")
        .arg(&new_output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("讀取"));
    assert!(!new_output.exists());

    // 有效程式寫到目錄時應明確回報 I/O 失敗。
    fs::write(&input, include_str!("../examples/hello.rs")).unwrap();
    let output_directory = dir.file("directory.c");
    fs::create_dir(&output_directory).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
        .arg(input)
        .arg("-o")
        .arg(output_directory)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("寫入"));
}

/// CLI 應檢查參數與副檔名，並提供可成功退出的 help。
#[test]
fn cli_checks_arguments_and_provides_help() {
    let help = successful(Command::new(env!("CARGO_BIN_EXE_idwc")).arg("--help"));
    assert!(String::from_utf8_lossy(&help.stdout).contains("idwc input.rs -o output.c"));
    for args in [
        vec![],
        vec!["input.rs"],
        vec!["input.rs", "-o"],
        vec!["input.rs", "--output", "output.c"],
        vec!["input.rs", "-o", "output.c", "extra"],
        vec!["input.txt", "-o", "output.c"],
        vec!["input.rs", "-o", "output.rs"],
        vec!["--help", "extra"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!result.status.success(), "應拒絕參數：{args:?}");
        assert!(!result.stderr.is_empty());
    }
}

/// 向前宣告、遞迴、值參數與呼叫副作用在 Rust 和 C 中保持一致。
#[test]
fn functions_match_rust_output() {
    let cases = [
        r#"fn main() { println!("{} {}", factorial(6), even(8)); }
        fn factorial(n: i32) -> i32 { if n <= 1 { return 1; } n * factorial(n - 1) }
        fn even(n: i32) -> bool { if n == 0 { return true; } odd(n - 1) }
        fn odd(n: i32) -> bool { if n == 0 { return false; } even(n - 1) }"#,
        r#"fn main() { let x = 3; println!("{} {}", bump(x), x); done(); }
        fn bump(mut x: i32) -> i32 { x += 2; { let x = false; println!("{}", x); } x }
        fn done() -> () { print!("done"); return (); }"#,
        r#"fn main() { println!("answer = {}", sum(mark(1), mark(2))); }
        fn mark(n: i32) -> i32 { print!("{} ", n); n }
        fn sum(a: i32, b: i32) -> i32 { a + b }"#,
        r#"fn main() { println!("{} {}", false && mark(), true || mark()); finish() }
        fn mark() -> bool { println!("unexpected"); true }
        fn finish() { println!("finished"); }"#,
        r#"fn main() { println!("{}", classify(false)); return; }
        fn classify(n: bool) -> i32 { if n { return 1; } else { return -1; } }"#,
        r#"fn main() { let mut n = 0; while active(n) { n += 1; continue; } println!("{}", n); }
        fn active(n: i32) -> bool { print!("{} ", n); n < 3 }"#,
        r#"fn main() { println!("{}", r#int(2)); }
        fn r#int(printf: i32) -> i32 { printf } fn unused(_n: bool) {}"#,
        "fn main() { return (); } fn unit() -> () { () }",
    ];
    let dir = TestDir::new();
    for source in cases {
        let (rust, c) = compile_pair(&dir, source);
        let rust = run_with_input(&rust, b"");
        let c = run_with_input(&c, b"");
        assert!(rust.status.success(), "{source}");
        assert_same_output(&rust, &c);
    }
}

/// 在相同 stdin 下比較多次讀取、格式拒絕、長度、EOF 與 i32 邊界。
#[test]
fn typed_stdin_matches_rust_output() {
    let source = r#"fn main() { print!("input: "); idwc::io::flush_stdout();
        let first = idwc::io::read_i32(); println!("first = {}", first);
        println!("second = {}", read_next()); }
        fn read_next() -> i32 { idwc::io::read_i32() }"#;
    let dir = TestDir::new();
    let (rust, c) = compile_pair(&dir, source);
    let mut cases: Vec<(Vec<u8>, Option<&str>)> = vec![
        (b"12 34".to_vec(), None),
        (
            b" \t\r\n\x0b\x0c+2147483647\n-2147483648\r\n".to_vec(),
            None,
        ),
        (b"-0 +007".to_vec(), None),
        (b"".to_vec(), Some("unexpected EOF")),
        (b" \t\n".to_vec(), Some("unexpected EOF")),
        (b"1".to_vec(), Some("unexpected EOF")),
        (b"12x 2".to_vec(), Some("invalid integer")),
        (b"1 2x".to_vec(), Some("invalid integer")),
        (b"+ 2".to_vec(), Some("invalid integer")),
        (b"- 2".to_vec(), Some("invalid integer")),
        (b"1.0 2".to_vec(), Some("invalid integer")),
        (b"0xff 2".to_vec(), Some("invalid integer")),
        (b"1_000 2".to_vec(), Some("invalid integer")),
        (b"1\0 2".to_vec(), Some("invalid integer")),
        (b"\xff 2".to_vec(), Some("invalid integer")),
        ("１２ 2".as_bytes().to_vec(), Some("invalid integer")),
        (b"2147483648 2".to_vec(), Some("integer out of range")),
        (b"-2147483649 2".to_vec(), Some("integer out of range")),
        (
            b"999999999999999999999x 2".to_vec(),
            Some("invalid integer"),
        ),
    ];
    let mut boundary = vec![b'0'; 128];
    boundary.extend_from_slice(b" 2");
    cases.push((boundary, None));
    cases.push((vec![b'0'; 129], Some("input token too long")));
    cases.push((vec![b'9'; 128], Some("integer out of range")));
    for (input, error) in cases {
        let rust = run_with_input(&rust, &input);
        let c = run_with_input(&c, &input);
        assert_same_output(&rust, &c);
        match error {
            None => assert!(c.status.success(), "{input:?}"),
            Some(message) => {
                assert_eq!(c.status.code(), Some(101), "{input:?}");
                assert_eq!(c.stderr, format!("idwc: {message}\n").as_bytes());
                assert!(c.stdout.starts_with(b"input: "));
            }
        }
    }
    // POSIX/macOS：讀取目錄會回報 I/O 錯誤，不能誤當 EOF。
    let run_directory = |binary: &Path| {
        run_with_timeout(Command::new(binary).stdin(Stdio::from(fs::File::open(&dir.0).unwrap())))
    };
    let rust_output = run_directory(&rust);
    let c_output = run_directory(&c);
    assert_same_output(&rust_output, &c_output);
    assert_eq!(c_output.stderr, b"idwc: stdin I/O error\n");
    assert_eq!(c_output.status.code(), Some(101));
}

#[test]
fn stdin_call_order_and_short_circuit_match_rust() {
    let dir = TestDir::new();
    let (rust, c) = compile_pair(
        &dir,
        r#"fn main() {
        println!("{}", subtract(idwc::io::read_i32(), idwc::io::read_i32()));
        println!("{} {}", false && (idwc::io::read_i32() == 0),
            true || (idwc::io::read_i32() == 0));
        println!("{}", idwc::io::read_i32()); }
        fn subtract(a: i32, b: i32) -> i32 { a - b }"#,
    );
    let rust = run_with_input(&rust, b"9 4 7");
    let c = run_with_input(&c, b"9 4 7");
    assert_same_output(&rust, &c);
    assert_eq!(c.stdout, b"5\nfalse true\n7\n");
    assert!(c.status.success());
}

/// stdin 尚未收到資料時就必須觀察到提示，避免換行掩蓋 flush 問題。
#[test]
fn interactive_prompt_is_flushed_before_input() {
    let dir = TestDir::new();
    let (rust, c) = compile_pair(&dir, include_str!("../examples/functions_stdin.rs"));
    for binary in [rust, c] {
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut prompt = [0; 20];
            let result = stdout.read_exact(&mut prompt);
            let _ = sender.send((result, prompt));
            let mut remaining = Vec::new();
            stdout.read_to_end(&mut remaining).unwrap();
            remaining
        });
        let prompt = receiver.recv_timeout(Duration::from_secs(2));
        if prompt.is_err() {
            child.kill().unwrap();
            let _ = child.wait();
            let _ = reader.join();
            panic!("提示未在等待輸入前 flush");
        }
        let (result, prompt) = prompt.unwrap();
        result.unwrap();
        assert_eq!(&prompt, b"Enter two integers: ");
        child.stdin.take().unwrap().write_all(b"3 4\n").unwrap();
        let output = wait_with_timeout(child);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(reader.join().unwrap(), b"sum = 7, positive = true\n");
    }
}

/// 明確 flush 的失敗必須回報診斷，不能回到 main 繼續執行。
#[test]
fn explicit_flush_reports_io_failure() {
    let dir = TestDir::new();
    let (rust, c) = compile_pair(
        &dir,
        r#"fn main() { print!("prompt");
        idwc::io::flush_stdout(); println!("unexpected"); }"#,
    );
    for binary in [rust, c] {
        let (writer, reader) = std::os::unix::net::UnixStream::pair().unwrap();
        drop(reader);
        let fd = std::os::fd::OwnedFd::from(writer);
        let output = Command::new(&binary)
            .stdout(Stdio::from(fd))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(101), "{binary:?}: {output:?}");
        assert_eq!(output.stderr, b"idwc: stdout flush error\n");
    }
}
