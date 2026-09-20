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
    // 同時排空 pipes，讓浮點 subnormal／大規模格式測試不被 pipe 容量卡住。
    let stdout = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).unwrap();
            bytes
        })
    });
    let stderr = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).unwrap();
            bytes
        })
    });
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return Output {
                status: child.wait().unwrap(),
                stdout: stdout.map_or_else(Vec::new, |reader| reader.join().unwrap()),
                stderr: stderr.map_or_else(Vec::new, |reader| reader.join().unwrap()),
            };
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
    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = wait_with_timeout(child);
    if let Err(error) = writer.join().unwrap() {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    output
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
            .arg("-lm")
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
fn c_compiler() -> String {
    if let Ok(compiler) = std::env::var("IDWC_CC") {
        if Command::new(&compiler)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return compiler;
        }
        panic!("IDWC_CC 指定的 C compiler 無法執行：{compiler}");
    }
    for compiler in ["clang", "gcc"] {
        if Command::new(compiler)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return compiler.into();
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
        include_str!("../examples/strings.rs"),
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

/// 固定陣列的初始化、複製、索引求值順序與修改應符合 Rust。
#[test]
fn fixed_arrays_match_rust_output() {
    let cases = [
        include_str!("../examples/arrays.rs"),
        r#"fn main() {
            let mut values = [mark(1), mark(2), mark(3)];
            let repeated = [mark(4); 3];
            values[index()] += mark(5);
            values = [values[1], values[0], values[2]];
            println!("| {} {} {} {}", values[0], values[1], values[2], repeated[2]);
        }
        fn mark(value: i32) -> i32 { print!("{} ", value); value }
        fn index() -> usize { print!("i "); 1 }"#,
        r#"fn main() {
            let mut numbers: [usize; 4] = [1, 2usize, 3, 4];
            let original = numbers;
            let mut i: usize = 0;
            while i < numbers.len() { numbers[i] *= 2; i += 1; }
            println!("{} {} {}", numbers[3], original[3], numbers.len());
        }
        "#,
        r#"fn main() {
            let floats = [1.25, -0.0, 3.5];
            let flags = [true, false, true];
            let empty: [i32; 0] = [];
            println!("{} {} {} {}", floats[0], floats[1], flags[2], empty.len());
        }"#,
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

#[test]
fn fixed_capacity_vecs_match_rust_output() {
    let cases = [
        include_str!("../examples/vectors.rs"),
        r#"fn main() {
            let mut values = vec![mark(1), mark(2), mark(3)];
            let repeated = vec![mark(4); 3];
            values.push(mark(5));
            values[index()] = mark(6);
            println!("| {} {} {} {}", values.len(), values[1], values[3], repeated[2]);
        }
        fn mark(value: i32) -> i32 { print!("{} ", value); value }
        fn index() -> usize { print!("i "); 1 }"#,
        r#"fn main() {
            let mut values: Vec<usize> = Vec::new();
            values.push(values.len());
            values.push(values.len());
            let moved = values;
            let mut replacement = vec![3usize; 2];
            replacement = moved;
            println!("{} {} {}", replacement.len(), replacement[0], replacement[1]);
        }"#,
        r#"fn main() {
            let floats = vec![1.25, -0.0, 3.5];
            let flags = vec![true, false, true];
            let empty: Vec<i32> = vec![];
            println!("{} {} {} {}", floats[0], floats[1], flags[2], empty.len());
        }"#,
        r#"fn main() {
            let array = [1usize, 2, 3];
            let values = vec![true, false, true];
            println!("{} array={:?} values={:?}", mark(7), array, values);
        }
        fn mark(value: i32) -> i32 { print!("mark "); value }"#,
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

#[test]
fn vec_bounds_and_capacity_fail_without_undefined_behavior() {
    let dir = TestDir::new();

    let bounds_source = r#"fn main() {
        let values = vec![10, 20, 30];
        println!("before");
        println!("{}", values[out_of_bounds()]);
    }
    fn out_of_bounds() -> usize { 3 }"#;
    let (rust, c) = compile_pair(&dir, bounds_source);
    let rust = run_with_input(&rust, b"");
    let c = run_with_input(&c, b"");
    assert_eq!(rust.status.code(), Some(101));
    assert_eq!(c.status.code(), Some(101));
    assert_eq!(c.stdout, b"before\n");
    assert_eq!(c.stderr, b"idwc: Vec index out of bounds\n");

    let capacity_source =
        "fn main() { let mut values = vec![1, 2]; println!(\"before\"); values.push(3); }";
    let c_source = dir.file("capacity.c");
    let c_binary = dir.file("capacity");
    let generated = idwc::transpile_with_options(
        capacity_source,
        idwc::TranspileOptions::default().with_vec_capacity(2),
    )
    .unwrap();
    fs::write(&c_source, generated).unwrap();
    compile_c(&c_source, &c_binary);
    let c = run_with_input(&c_binary, b"");
    assert_eq!(c.status.code(), Some(101));
    assert_eq!(c.stdout, b"before\n");
    assert_eq!(c.stderr, b"idwc: Vec capacity exceeded\n");
}

/// 動態越界必須在存取前停止，且 C 端不得觸發未定義行為。
#[test]
fn array_bounds_fail_without_undefined_behavior() {
    let cases = [
        (
            r#"fn main() {
                let values = [10, 20, 30];
                println!("before");
                println!("{}", values[out_of_bounds()]);
            }
            fn out_of_bounds() -> usize { 3 }"#,
            b"before\n".as_slice(),
        ),
        (
            r#"fn main() {
                let mut values = [10, 20, 30];
                println!("before");
                values[out_of_bounds()] = right_hand_side();
            }
            fn right_hand_side() -> i32 { println!("right"); 40 }
            fn out_of_bounds() -> usize { println!("index"); 3 }"#,
            b"before\nright\nindex\n".as_slice(),
        ),
        (
            r#"fn main() {
                let values: [i32; 0] = [];
                println!("before");
                println!("{}", values[out_of_bounds()]);
            }
            fn out_of_bounds() -> usize { 0 }"#,
            b"before\n".as_slice(),
        ),
    ];
    let dir = TestDir::new();
    for (source, expected_stdout) in cases {
        let (rust, c) = compile_pair(&dir, source);
        let rust = run_with_input(&rust, b"");
        let c = run_with_input(&c, b"");
        assert_eq!(rust.status.code(), Some(101));
        assert_eq!(c.status.code(), Some(101));
        assert_eq!(rust.stdout, expected_stdout);
        assert_eq!(c.stdout, rust.stdout);
        assert_eq!(c.stderr, b"idwc: array index out of bounds\n");
    }
}

/// 限定行輸入流程的 UTF-8、Unicode whitespace、附加、解析與 powi(2) 應符合 Rust。
#[test]
fn line_input_parsing_and_powi_match_rust() {
    let cases = [
        (
            include_str!("../examples/line_input.rs"),
            "70\u{2003}1.75\n",
        ),
        (
            r#"fn main() {
                let input = idwc::io::read_line();
                let value = input.trim().parse::<i32>().unwrap();
                println!("{}", value);
            }"#,
            "\u{00a0}-42\u{3000}\n",
        ),
        (
            r#"fn main() {
                let first = idwc::io::read_line();
                let second: String = idwc::io::read_line();
                let first_values: Vec<&str> = first.split_whitespace().collect();
                let second_values: Vec<&str> = second.split_whitespace().collect();
                println!("{} {}", first_values.len(), second_values.len());
            }"#,
            "1 2\n3 4 5\n",
        ),
        (
            r#"fn main() {
                let input = idwc::io::read_line();
                let values: Vec<&str> = input.split_whitespace().collect();
                println!("{}", values.len());
            }"#,
            "",
        ),
        (
            r#"fn main() {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let value = input.trim().parse::<i32>().unwrap();
                println!("{}", value);
            }"#,
            "\u{00a0}-42\u{3000}\n",
        ),
        (
            r#"fn main() {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                std::io::stdin().read_line(&mut input).unwrap();
                let values: Vec<&str> = input.split_whitespace().collect();
                println!("{} {} {} {} {}", values.len(),
                    values[0].parse::<i32>().unwrap(), values[1].parse::<i32>().unwrap(),
                    values[2].parse::<i32>().unwrap(), values[3].parse::<i32>().unwrap());
            }"#,
            "1 2\n3 4\n",
        ),
        (
            r#"fn main() {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let values = input.split_whitespace().collect::<Vec<&str>>();
                println!("{}", values.len());
            }"#,
            "",
        ),
        (
            r#"fn main() {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let values: Vec<&str> = input.split_whitespace().collect();
                let a = values[0].parse::<f64>().unwrap();
                let b = values[1].parse::<f64>().unwrap();
                let c = values[2].parse::<f64>().unwrap();
                let d = values[3].parse::<f64>().unwrap();
                println!("{} {} {} {}", a.powi(2), b.powi(2), c.powi(2), d.powi(2));
            }"#,
            "-0 NaN 1e9999 1e-9999\n",
        ),
    ];
    let dir = TestDir::new();
    for (source, input) in cases {
        let (rust, c) = compile_pair(&dir, source);
        let rust = run_with_input(&rust, input.as_bytes());
        let c = run_with_input(&c, input.as_bytes());
        assert!(rust.status.success(), "{source}: {rust:?}");
        assert_same_output(&rust, &c);
    }
}

/// 行輸入的所有失敗路徑都必須在存取 C buffer 前受控結束。
#[test]
fn line_input_failures_are_controlled() {
    let integer_source = r#"fn main() {
        print!("input: "); idwc::io::flush_stdout();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let values: Vec<&str> = input.split_whitespace().collect();
        println!("{}", values[1].parse::<i32>().unwrap());
    }"#;
    let float_source = r#"fn main() {
        print!("input: "); idwc::io::flush_stdout();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let values: Vec<&str> = input.split_whitespace().collect();
        println!("{}", values[0].parse::<f64>().unwrap());
    }"#;
    let dir = TestDir::new();
    for (source, input, diagnostic) in [
        (integer_source, b"1 x\n".as_slice(), "invalid integer"),
        (
            integer_source,
            b"1 2147483648\n".as_slice(),
            "integer out of range",
        ),
        (
            integer_source,
            b"1\n".as_slice(),
            "token index out of bounds",
        ),
        (integer_source, b"1 2\0\n".as_slice(), "invalid integer"),
        (float_source, b"1.2x\n".as_slice(), "invalid float"),
    ] {
        let (rust, c) = compile_pair(&dir, source);
        let rust = run_with_input(&rust, input);
        let c = run_with_input(&c, input);
        assert_eq!(rust.status.code(), Some(101));
        assert_eq!(c.status.code(), Some(101));
        assert_eq!(c.stdout, rust.stdout);
        assert_eq!(c.stderr, format!("idwc: {diagnostic}\n").as_bytes());
    }

    let read_source = r#"fn main() {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        println!("read");
    }"#;
    let (rust, c) = compile_pair(&dir, read_source);
    let rust_invalid_utf8 = run_with_input(&rust, b"\xff\n");
    let invalid_utf8 = run_with_input(&c, b"\xff\n");
    assert_eq!(rust_invalid_utf8.status.code(), Some(101));
    assert_eq!(invalid_utf8.status.code(), Some(101));
    assert_eq!(invalid_utf8.stdout, rust_invalid_utf8.stdout);
    assert_eq!(invalid_utf8.stderr, b"idwc: invalid UTF-8 input\n");

    let exact_limit = run_with_input(&c, &vec![b'a'; 4096]);
    assert!(exact_limit.status.success());
    assert_eq!(exact_limit.stdout, b"read\n");
    let over_limit = run_with_input(&c, &vec![b'a'; 4097]);
    assert_eq!(over_limit.status.code(), Some(101));
    assert_eq!(over_limit.stderr, b"idwc: input line buffer too long\n");

    let run_directory = |binary: &Path| {
        run_with_timeout(Command::new(binary).stdin(Stdio::from(fs::File::open(&dir.0).unwrap())))
    };
    let rust_io_error = run_directory(&rust);
    let c_io_error = run_directory(&c);
    assert_eq!(rust_io_error.status.code(), Some(101));
    assert_eq!(c_io_error.status.code(), Some(101));
    assert_eq!(c_io_error.stdout, rust_io_error.stdout);
    assert_eq!(c_io_error.stderr, b"idwc: stdin I/O error\n");

    let convenience_source = r#"fn main() {
        let input = idwc::io::read_line();
        let values: Vec<&str> = input.split_whitespace().collect();
        println!("{}", values.len());
    }"#;
    let (rust, c) = compile_pair(&dir, convenience_source);
    let rust_output = run_with_input(&rust, b"\xff\n");
    let c_output = run_with_input(&c, b"\xff\n");
    assert_eq!(rust_output.status.code(), Some(101));
    assert_eq!(c_output.status.code(), Some(101));
    assert_eq!(c_output.stdout, rust_output.stdout);
    assert_eq!(c_output.stderr, b"idwc: invalid UTF-8 input\n");

    let over_limit_input = vec![b'a'; 4097];
    let rust_output = run_with_input(&rust, &over_limit_input);
    let c_output = run_with_input(&c, &over_limit_input);
    assert_eq!(rust_output.status.code(), Some(101));
    assert_eq!(c_output.status.code(), Some(101));
    assert_eq!(c_output.stdout, rust_output.stdout);
    assert_eq!(c_output.stderr, b"idwc: input line buffer too long\n");

    let rust_io_error = run_directory(&rust);
    let c_io_error = run_directory(&c);
    assert_eq!(rust_io_error.status.code(), Some(101));
    assert_eq!(c_io_error.status.code(), Some(101));
    assert_eq!(c_io_error.stdout, rust_io_error.stdout);
    assert_eq!(c_io_error.stderr, b"idwc: stdin I/O error\n");

    let token_source = r#"fn main() {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let values: Vec<&str> = input.split_whitespace().collect();
        println!("{}", values.len());
    }"#;
    let (_, c) = compile_pair(&dir, token_source);
    let input_2048 = vec!["1"; 2048].join(" ");
    let output = run_with_input(&c, input_2048.as_bytes());
    assert!(output.status.success());
    assert_eq!(output.stdout, b"2048\n");
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
        r#"fn main() {
            println!();
            let mut sum = 0;
            for i in -2..3 {
                if i == 0 { continue; }
                sum += i;
            }
            for mut i in 0..3 {
                i += 10;
                sum += i;
            }
            for i in 3..3 { sum += i; }
            for i in 5..2 { sum += i; }
            println!("{}", sum);
        }"#,
        r#"fn main() {
            for i in 2147483646..=2147483647 {
                if i == 2147483647 { continue; }
                println!("{}", i);
            }
            let mut total: usize = 0;
            for i in 0usize..=2usize { total += i; }
            println!("{}", total);
        }"#,
        r#"fn start() -> i32 { println!("start"); 1 }
        fn end() -> i32 { println!("end"); 4 }
        fn main() {
            for i in start()..end() {
                if i == 2 { continue; }
                println!("{}", i);
            }
        }"#,
        r#"fn main() {
            let mut count = 0;
            for outer in 0..3 {
                for inner in 0..4 {
                    if inner == 1 { continue; }
                    if outer == 2 { break; }
                    count += 1;
                }
            }
            println!("{}", count);
        }"#,
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

/// Inclusive usize range 在 target 最大值停止，不執行溢位 increment。
#[test]
fn integer_range_usize_max_matches_rust() {
    let source = format!(
        "fn main() {{ for value in {}usize..={}usize {{ println!(\"{{}}\", value); continue; }} }}",
        usize::MAX - 1,
        usize::MAX
    );
    let dir = TestDir::new();
    let (rust, c) = compile_pair(&dir, &source);
    let rust_output = run_with_timeout(&mut Command::new(rust));
    let c_output = run_with_timeout(&mut Command::new(c));
    assert_same_output(&rust_output, &c_output);
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
        ("let x: usize = 0; let y = x - 1;", "integer overflow"),
        ("let mut x: usize = 0; x -= 1;", "integer overflow"),
        (
            "let zero: usize = 0; let y = 1usize / zero;",
            "division by zero",
        ),
        (
            "let zero: usize = 0; let y = 1usize % zero;",
            "division by zero",
        ),
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
    fs::write(&output, "stale output").unwrap();
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

    let default_input = dir.file("default output.rs");
    let default_output = dir.file("default output.c");
    let default_executable = dir.file("default-output");
    fs::write(&default_input, include_str!("../examples/hello.rs")).unwrap();
    successful(Command::new(env!("CARGO_BIN_EXE_idwc")).arg(&default_input));
    assert!(default_output.exists());
    compile_c(&default_output, &default_executable);
    assert_eq!(
        successful(&mut Command::new(default_executable)).stdout,
        b"Hello, World!\n"
    );
}

/// 錯誤輸入不應建立或覆寫輸出檔案，且應提供非零退出狀態。
#[test]
fn cli_reports_errors_without_overwriting_output() {
    let dir = TestDir::new();
    let input = dir.file("invalid.rs");
    let output = dir.file("output.c");
    fs::write(&input, "fn main() { let x = 1.0f32; }").unwrap();
    fs::write(&output, "existing output").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("不支援的語法"));
    assert!(stderr.contains("第 1 行"));
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
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("idwc [選項] <input.rs> [-o <output.c>]"));
    assert!(help.contains("省略時使用輸入檔名並改為 .c"));
    assert!(help.contains("-s, --stupid"));
    assert!(help.contains("放棄部分嚴格語意保證"));
    assert!(help.contains("--string-capacity <B>"));
    assert!(help.contains("--vec-capacity <N>"));
    let short_help = successful(Command::new(env!("CARGO_BIN_EXE_idwc")).arg("-h"));
    assert_eq!(String::from_utf8_lossy(&short_help.stdout), help);
    let version = successful(Command::new(env!("CARGO_BIN_EXE_idwc")).arg("--version"));
    assert_eq!(version.stdout, b"idwc 1.2.1\n");
    for args in [
        vec![],
        vec!["input.rs", "-o"],
        vec!["input.rs", "--output", "output.c"],
        vec!["input.rs", "-o", "output.c", "extra"],
        vec!["input.txt", "-o", "output.c"],
        vec!["input.rs", "-o", "output.rs"],
        vec!["--help", "extra"],
        vec!["--version", "extra"],
        vec!["--stupid", "--stupid", "input.rs", "-o", "output.c"],
        vec!["--string-capacity", "0", "input.rs"],
        vec!["--string-capacity", "65537", "input.rs"],
        vec!["--string-capacity", "many", "input.rs"],
        vec![
            "--string-capacity",
            "8",
            "--string-capacity",
            "9",
            "input.rs",
        ],
        vec!["--vec-capacity", "0", "input.rs"],
        vec!["--vec-capacity", "65537", "input.rs"],
        vec!["--vec-capacity", "many", "input.rs"],
        vec!["--vec-capacity", "3", "--vec-capacity", "4", "input.rs"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_idwc"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!result.status.success(), "應拒絕參數：{args:?}");
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn cli_applies_collection_capacities() {
    let dir = TestDir::new();
    let input = dir.file("line.rs");
    let output = dir.file("line.c");
    fs::write(
        &input,
        "fn main() { let input = idwc::io::read_line(); let values: Vec<&str> = input.split_whitespace().collect(); println!(\"{}\", values.len()); }",
    )
    .unwrap();
    successful(
        Command::new(env!("CARGO_BIN_EXE_idwc"))
            .args(["--string-capacity", "8", "--vec-capacity", "3"])
            .arg(&input)
            .arg("-o")
            .arg(&output),
    );
    let generated = fs::read_to_string(&output).unwrap();
    assert!(generated.contains("#define IDWC_LINE_LIMIT ((size_t)8)"));
    assert!(generated.contains("#define IDWC_TOKEN_LIMIT ((size_t)3)"));

    let executable = dir.file("line");
    compile_c(&output, &executable);
    let accepted = run_with_input(&executable, b"a b c\n");
    assert!(accepted.status.success());
    assert_eq!(accepted.stdout, b"3\n");
    let rejected = run_with_input(&executable, b"a b c d\n");
    assert_eq!(rejected.status.code(), Some(101));
    assert_eq!(rejected.stderr, b"idwc: too many input tokens\n");
}

#[test]
fn string_capacity_overflow_is_controlled() {
    let dir = TestDir::new();
    let c_source = dir.file("string.c");
    let executable = dir.file("string");
    let options = idwc::TranspileOptions::default().with_string_capacity(8);

    let accepted = idwc::transpile_with_options(
        "fn main() { let value = String::from(\"12345678\"); println!(\"{}\", value); }",
        options,
    )
    .unwrap();
    fs::write(&c_source, accepted).unwrap();
    compile_c(&c_source, &executable);
    let output = run_with_timeout(&mut Command::new(&executable));
    assert!(output.status.success());
    assert_eq!(output.stdout, b"12345678\n");

    let rejected = idwc::transpile_with_options(
        "fn main() { let value = String::from(\"123456789\"); println!(\"{}\", value); }",
        options,
    )
    .unwrap();
    fs::write(&c_source, rejected).unwrap();
    compile_c(&c_source, &executable);
    let output = run_with_timeout(&mut Command::new(&executable));
    assert_eq!(output.status.code(), Some(101));
    assert_eq!(output.stderr, b"idwc: String capacity exceeded\n");
}

/// CLI Stupid Mode should emit readable standalone C and a visible semantic warning.
#[test]
fn stupid_mode_cli_generates_readable_c() {
    let dir = TestDir::new();
    let input = dir.file("bmi.rs");
    let output = dir.file("bmi.c");
    let executable = dir.file("bmi");
    fs::write(&input, include_str!("../examples/stupid_stdin.rs")).unwrap();

    let translated = successful(
        Command::new(env!("CARGO_BIN_EXE_idwc"))
            .arg("--stupid")
            .arg(&input)
            .arg("-o")
            .arg(&output),
    );
    assert!(String::from_utf8_lossy(&translated.stderr).contains("--stupid"));
    let generated = fs::read_to_string(&output).unwrap();
    assert!(generated.contains("double kg;"));
    assert!(generated.contains("puts(u8\"過瘦\");"));
    assert!(!generated.contains("idwc_fail"));
    assert!(!generated.contains("struct "));

    compile_c(&output, &executable);
    let result = run_with_input(&executable, b"70 175\n");
    assert!(result.status.success());
    assert_eq!(result.stdout, "標準\n".as_bytes());
    assert!(result.stderr.is_empty());

    let short_output = dir.file("bmi-short.c");
    successful(
        Command::new(env!("CARGO_BIN_EXE_idwc"))
            .arg("-s")
            .arg(&input)
            .arg("-o")
            .arg(&short_output),
    );
    assert_eq!(fs::read(&output).unwrap(), fs::read(short_output).unwrap());
}

/// Every checked-in Rust example should remain valid C17 under the relaxed generator.
#[test]
fn stupid_mode_examples_compile() {
    let dir = TestDir::new();
    for (name, source) in [
        ("hello", include_str!("../examples/hello.rs")),
        ("variables", include_str!("../examples/variables.rs")),
        ("control_flow", include_str!("../examples/control_flow.rs")),
        (
            "functions_stdin",
            include_str!("../examples/functions_stdin.rs"),
        ),
        (
            "floating_point",
            include_str!("../examples/floating_point.rs"),
        ),
        ("arrays", include_str!("../examples/arrays.rs")),
        ("line_input", include_str!("../examples/line_input.rs")),
        ("ranges", include_str!("../examples/ranges.rs")),
        ("strings", include_str!("../examples/strings.rs")),
        ("vectors", include_str!("../examples/vectors.rs")),
        ("stupid_stdin", include_str!("../examples/stupid_stdin.rs")),
    ] {
        let c_source = dir.file(&format!("{name}.c"));
        let executable = dir.file(name);
        let generated = idwc::transpile_with_options(
            source,
            idwc::TranspileOptions::new(idwc::TranspileMode::Stupid),
        )
        .unwrap();
        fs::write(&c_source, generated).unwrap();
        compile_c(&c_source, &executable);
    }
}

#[test]
fn stupid_mode_string_and_vec_output_compiles_and_runs() {
    let source = r#"fn main() {
        let original = "first";
        let mut alias = original;
        alias = "second";
        let mut values = vec![1, 2];
        values.push(3);
        values[0] = 4;
        let replacement = vec![5, 6];
        values = replacement;
        println!("{} {:?} {}", alias, values, values.len());
    }"#;
    let dir = TestDir::new();
    let c_source = dir.file("stupid-collections.c");
    let executable = dir.file("stupid-collections");
    let generated = idwc::transpile_with_options(
        source,
        idwc::TranspileOptions::new(idwc::TranspileMode::Stupid),
    )
    .unwrap();
    fs::write(&c_source, generated).unwrap();
    compile_c(&c_source, &executable);
    let output = run_with_timeout(&mut Command::new(&executable));
    assert!(output.status.success());
    assert_eq!(output.stdout, b"second [5, 6] 2\n");
    assert!(output.stderr.is_empty());
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

/// 數值用絕對／相對誤差比對；格式与特殊值另採精確輸出比對。
#[test]
fn floating_point_operations_and_formats_match_rust() {
    let dir = TestDir::new();
    let source = r#"fn main() {
        let mut n: f64 = 1.5; n += 2.25; n -= 0.5; n *= 3.0; n /= 2.0;
        println!("{} {:.2} {:.18}", n, n, -n);
        println!("{} {} {} {} {} {}", n < 5.0, n <= 5.0, n > 5.0, n >= 5.0, n == 5.0, n != 5.0);
        println!("{} {}", divide(1f64, 3f64), multiply(0.1, 0.2));
        println!("{:.0} {:.0} {:.2} {:.2}", 2.5, 3.5, 1.125, 1.375);
        println!("{} {} {:.2} {:.0}", -0.0, 0.0, -0.0, -0.0);
        let zero = 0.0; let nan = zero / zero;
        println!("{} {} {} {:.2} {:.2}", 1.0 / zero, -1.0 / zero, nan, nan, 1.0 / -zero);
        println!("{} {} {} {}", nan == nan, nan != nan, nan < 1.0, nan >= 1.0);
        println!("{} {} {}", 1e308 * 1e308, 5e-324 / 2.0, 1.0 / -0.0);
        println!("{} {}", false && (divide(1.0, zero) < 0.0), true || (nan == nan));
    }
    fn divide(a: f64, b: f64) -> f64 { a / b }
    fn multiply(a: f64, b: f64) -> f64 { a * b }"#;
    let (rust, c) = compile_pair(&dir, source);
    let rust = run_with_input(&rust, b"");
    let c = run_with_input(&c, b"");
    assert!(c.status.success(), "{c:?}");
    assert_same_output(&rust, &c);
    let text = String::from_utf8(c.stdout).unwrap();
    let numbers = text
        .lines()
        .nth(2)
        .unwrap()
        .split_whitespace()
        .map(|number| number.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    for (actual, expected) in numbers.iter().zip([1.0 / 3.0, 0.02]) {
        assert!((actual - expected).abs() <= 1e-15 + 1e-14 * expected.abs());
    }
}

/// 固定的偽隨機 bit patterns、十進位邊界與 subnormal，比對最短及固定格式。
#[test]
fn floating_point_format_boundaries_match_rust() {
    let dir = TestDir::new();
    let source = r#"fn main() { let count = idwc::io::read_i32(); let mut n = 0;
        while n < count { let x = idwc::io::read_f64(); println!("{} {:.2} {:.18}", x, x, x); n += 1; } }"#;
    let (rust, c) = compile_pair(&dir, source);
    let mut values = vec![
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(2),
        f64::from_bits((1 << 52) - 1),
        0.1,
        0.3,
        1e-6,
        1e-7,
        1e20,
        1e23,
        1.2345678901234567,
        9.999999999999998,
        999999999999999.9,
        0.0,
        -0.0,
        // 這些精確可表示的中點刻意區分最短格式與固定精度的 tie 規則。
        1e15 + 0.25,
        -(1e15 + 0.25),
        1e15 + 0.75,
        1e14 + 0.125,
        1e14 + 0.625,
    ];
    for exponent in -1022..=1023 {
        if exponent % 31 == 0 {
            let bits = ((exponent + 1023) as u64) << 52;
            values.extend([
                f64::from_bits(bits - 1),
                f64::from_bits(bits),
                f64::from_bits(bits + 1),
            ]);
        }
    }
    let mut state = 0x4d595df4d0f33173u64;
    for _ in 0..512 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let value = f64::from_bits(state);
        if value.is_finite() {
            values.push(value);
        }
    }
    let mut input = format!("{}\n", values.len());
    for value in values {
        input.push_str(&format!("{value:.17e}\n"));
    }
    let rust = run_with_input(&rust, input.as_bytes());
    let c = run_with_input(&c, input.as_bytes());
    assert!(c.status.success(), "{c:?}");
    // 行別診斷避免一次印出所有巨大的十進位表示。
    assert_eq!(rust.status.code(), c.status.code());
    assert_eq!(rust.stderr, c.stderr);
    let rust_text = String::from_utf8(rust.stdout).unwrap();
    let c_text = String::from_utf8(c.stdout).unwrap();
    assert_eq!(rust_text.lines().count(), c_text.lines().count());
    for (index, (rust, c)) in rust_text.lines().zip(c_text.lines()).enumerate() {
        assert_eq!(rust, c, "格式案例 {index}");
    }
}

#[test]
fn floating_point_stdin_matches_rust() {
    let dir = TestDir::new();
    let (rust, c) = compile_pair(
        &dir,
        r#"fn main() { print!("input: ");
        idwc::io::flush_stdout(); let x = idwc::io::read_f64(); println!("{} {:.2}", x, x);
        println!("{}", idwc::io::read_i32()); }"#,
    );
    let mut cases: Vec<(Vec<u8>, Option<&str>)> = vec![
        (b"+.5 1".to_vec(), None),
        (b"-1.25e+2\n-2".to_vec(), None),
        (b"1. 3".to_vec(), None),
        (b"-0e9999 4".to_vec(), None),
        (b"NaN 5".to_vec(), None),
        (b"+inf 6".to_vec(), None),
        (b"-inf 7".to_vec(), None),
        (b"inf 8".to_vec(), None),
        (b"5e-324 9".to_vec(), None),
        (b"1.7976931348623157e308 10".to_vec(), None),
        (b"".to_vec(), Some("unexpected EOF")),
        (b" \t\n".to_vec(), Some("unexpected EOF")),
        (b"1.5".to_vec(), Some("unexpected EOF")),
        (b"1e309 1".to_vec(), Some("float out of range")),
        (b"-1e9999 1".to_vec(), Some("float out of range")),
        (b"1e-9999 1".to_vec(), Some("float out of range")),
        (b"2e-324 1".to_vec(), Some("float out of range")),
    ];
    for input in [
        "+", ".", "1e", "1e+", "1e-", "1.2x", "1.2.3", "0x1p2", "nan", "Infinity", "1_0", "１",
        "1\0",
    ] {
        cases.push((format!("{input} 1").into_bytes(), Some("invalid float")));
    }
    let mut boundary = vec![b'0'; 128];
    boundary.extend_from_slice(b" 1");
    cases.push((boundary, None));
    cases.push((vec![b'0'; 129], Some("input token too long")));
    for (input, error) in cases {
        let rust = run_with_input(&rust, &input);
        let c = run_with_input(&c, &input);
        assert_same_output(&rust, &c);
        match error {
            None => assert!(c.status.success(), "{input:?}"),
            Some(error) => {
                assert_eq!(c.status.code(), Some(101));
                assert_eq!(c.stderr, format!("idwc: {error}\n").as_bytes());
            }
        }
    }
    let (rust, c) = compile_pair(&dir, include_str!("../examples/floating_point.rs"));
    let rust = run_with_input(&rust, b"70 1.75\n");
    let c = run_with_input(&c, b"70 1.75\n");
    assert_same_output(&rust, &c);
    assert_eq!(
        c.stdout,
        b"Enter weight (kg) and height (m): BMI = 22.86, below 25 = true\n"
    );
}

#[test]
fn generated_floating_point_c_rejects_fast_math() {
    let dir = TestDir::new();
    let source = dir.file("fast.c");
    fs::write(
        &source,
        idwc::transpile("fn main() { let x = 1.0; }").unwrap(),
    )
    .unwrap();
    let output = Command::new(c_compiler())
        .args(["-std=c17", "-ffast-math"])
        .arg(source)
        .arg("-lm")
        .arg("-o")
        .arg(dir.file("fast"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires fast-math to be disabled"));
}
