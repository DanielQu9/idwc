//! 將經過白名單驗證的 Rust 子集轉成獨立的 C17 程式。
//! v0.5.0 支援 i32／f64／bool、控制流程、函式與有限 stdin，入口為 [`transpile`]。

mod codegen;
mod format;
pub mod io;
mod ir;
mod semantic;
mod validate;

use std::fmt;

/// 區分 Rust 解析失敗、未支援語法與語意錯誤。
#[derive(Debug)]
pub enum TranspileError {
    /// 原始碼無法解析成 Rust AST。
    Parse(syn::Error),
    /// 語法或字串內容超出目前的支援範圍。
    Unsupported(&'static str),
    /// 名稱解析、型別、可變性或整數字面量錯誤。
    Semantic(String),
}

impl fmt::Display for TranspileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "Rust 解析失敗：{error}"),
            Self::Unsupported(reason) => write!(f, "不支援的語法：{reason}"),
            Self::Semantic(reason) => write!(f, "語意錯誤：{reason}"),
        }
    }
}

impl std::error::Error for TranspileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Unsupported(_) | Self::Semantic(_) => None,
        }
    }
}

/// 將 i32／f64／bool、控制流程、函式與有限輸入輸出的 Rust 子集轉成 C17 原始碼。
/// 不會執行輸入原始碼或展開使用者巨集。
///
/// # Errors
/// 無效 Rust、未支援 AST／格式、語意錯誤或內嵌 NUL 會回傳錯誤。
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
