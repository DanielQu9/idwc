use std::{env, error::Error, fs, path::PathBuf, process::ExitCode};

const USAGE: &str = "用法：idwc input.rs -o output.c";

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
    let mut args = env::args_os().skip(1);
    let input = args.next().ok_or(USAGE)?;
    if input == "--help" || input == "-h" {
        if args.next().is_some() {
            return Err(USAGE.into());
        }
        println!("{USAGE}");
        return Ok(());
    }
    if args.next().as_deref() != Some(std::ffi::OsStr::new("-o")) {
        return Err(USAGE.into());
    }
    let output = args.next().ok_or(USAGE)?;
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let input = PathBuf::from(input);
    let output = PathBuf::from(output);
    if input.extension().is_none_or(|ext| ext != "rs")
        || output.extension().is_none_or(|ext| ext != "c")
    {
        return Err("輸入副檔名必須為 .rs，輸出副檔名必須為 .c".into());
    }
    let source = fs::read_to_string(&input)
        .map_err(|error| format!("讀取 {} 失敗：{error}", input.display()))?;
    let generated = idwc::transpile(&source)?;
    fs::write(&output, generated)
        .map_err(|error| format!("寫入 {} 失敗：{error}", output.display()))?;
    Ok(())
}
