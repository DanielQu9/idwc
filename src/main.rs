use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

const USAGE: &str =
    "用法：idwc [-s|--stupid] input.rs -o output.c\n       idwc --help\n       idwc --version";

/// 將 CLI 錯誤寫入 stderr，並回傳非零退出狀態。
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("idwc: {error}");
            ExitCode::FAILURE
        }
    }
}

/// 解析最小 CLI、讀取 Rust 並在轉譯成功後寫入 C 檔案。
fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        if args.len() != 1 {
            return Err(USAGE.into());
        }
        println!("{USAGE}");
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--version" || arg == "-V")
    {
        if args.len() != 1 {
            return Err(USAGE.into());
        }
        println!("idwc {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut stupid = false;
    let mut positional = Vec::new();
    for arg in args {
        if arg == "--stupid" || arg == "-s" {
            if stupid {
                return Err(USAGE.into());
            }
            stupid = true;
        } else {
            positional.push(arg);
        }
    }
    if positional.len() != 3 || positional[1] != "-o" {
        return Err(USAGE.into());
    }
    let input = PathBuf::from(&positional[0]);
    let output = PathBuf::from(&positional[2]);
    if input.extension().is_none_or(|ext| ext != "rs")
        || output.extension().is_none_or(|ext| ext != "c")
    {
        return Err("輸入副檔名必須為 .rs，輸出副檔名必須為 .c".into());
    }
    let source = fs::read_to_string(&input)
        .map_err(|error| format!("讀取 {} 失敗：{error}", input.display()))?;
    let options = idwc::TranspileOptions::new(if stupid {
        idwc::TranspileMode::Stupid
    } else {
        idwc::TranspileMode::Strict
    });
    let generated = idwc::transpile_with_options(&source, options)?;
    if stupid {
        eprintln!("idwc: warning: --stupid prioritizes readable C over strict Rust semantics");
    }
    write_atomic(&output, generated.as_bytes())
        .map_err(|error| format!("寫入 {} 失敗：{error}", output.display()))?;
    Ok(())
}

/// 先完整寫入同目錄暫存檔，再以 rename 取代目標，避免部分輸出。
fn write_atomic(output: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    for attempt in 0..100 {
        let temporary = parent.join(format!(".idwc-{}-{attempt}.tmp", std::process::id()));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        drop(file);
        if let Err(error) = fs::rename(&temporary, output) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "無法配置輸出暫存檔",
    ))
}
