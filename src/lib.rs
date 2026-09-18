//! 將經過白名單驗證的 Rust 子集轉成獨立的 C17 程式。
//! v0.1.0 僅接受 main 內的一次字串 println!，入口為 [`transpile`]。

mod codegen;
mod ir;
mod validate;

use std::fmt;

/// 區分 Rust 解析失敗與超出目前支援範圍的語法。
#[derive(Debug)]
pub enum TranspileError {
    /// 原始碼無法解析成 Rust AST。
    Parse(syn::Error),
    /// 語法或字串內容超出 v0.1.0 的支援範圍。
    Unsupported(&'static str),
}

impl fmt::Display for TranspileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "Rust 解析失敗：{error}"),
            Self::Unsupported(reason) => write!(f, "不支援的語法：{reason}"),
        }
    }
}

impl std::error::Error for TranspileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Unsupported(_) => None,
        }
    }
}

/// 將單一 main 與單一字串 println! 轉成 C17 原始碼。
/// 不會執行輸入原始碼或展開使用者巨集。
///
/// # Errors
/// 無效 Rust、未支援 AST、格式參數或內嵌 NUL 字元會回傳錯誤。
///
/// # Examples
/// ```
/// let c = idwc::transpile(r#"fn main() { println!("Hello, World!"); }"#)?;
/// assert!(c.contains("puts(\"Hello, World!\");"));
/// # Ok::<(), idwc::TranspileError>(())
/// ```
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let ast = syn::parse_file(source).map_err(TranspileError::Parse)?;
    let program = validate::lower(&ast)?;
    Ok(codegen::generate(&program))
}
