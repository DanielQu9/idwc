use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
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
    fs::write(&input, "fn main() {}").unwrap();
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
