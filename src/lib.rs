//! 將經過白名單驗證的 Rust 子集轉成獨立的 C17 程式。
//! v0.9.0 穩定 scalar、固定陣列、限定行輸入、整數 range、控制流程與函式，入口為 [`transpile`]。

mod codegen;
mod format;
pub mod io;
mod ir;
mod semantic;
mod validate;

use std::fmt;

/// 轉譯失敗的穩定分類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranspileErrorKind {
    /// 原始碼無法解析成 Rust AST。
    Parse,
    /// 語法或字串內容超出目前的支援範圍。
    Unsupported,
    /// 名稱解析、型別、可變性或字面量不符合子集規則。
    Semantic,
}

impl TranspileErrorKind {
    fn label(self) -> &'static str {
        match self {
            Self::Parse => "Rust 解析失敗",
            Self::Unsupported => "不支援的語法",
            Self::Semantic => "語意錯誤",
        }
    }
}

/// 原始碼中的一基準行號與欄號。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceLocation {
    /// 一基準行號。
    pub line: usize,
    /// 一基準欄號。
    pub column: usize,
}

impl SourceLocation {
    fn from_span(span: proc_macro2::Span) -> Self {
        let start = span.start();
        Self {
            line: start.line,
            column: start.column + 1,
        }
    }
}

/// 包含穩定分類、訊息及可用時的原始碼位置。
#[derive(Debug)]
pub struct TranspileError {
    kind: TranspileErrorKind,
    message: String,
    location: Option<SourceLocation>,
    parse_source: Option<syn::Error>,
}

impl TranspileError {
    fn parse(error: syn::Error) -> Self {
        Self {
            kind: TranspileErrorKind::Parse,
            message: error.to_string(),
            location: Some(SourceLocation::from_span(error.span())),
            parse_source: Some(error),
        }
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: TranspileErrorKind::Unsupported,
            message: message.into(),
            location: None,
            parse_source: None,
        }
    }

    pub(crate) fn semantic(message: impl Into<String>) -> Self {
        Self {
            kind: TranspileErrorKind::Semantic,
            message: message.into(),
            location: None,
            parse_source: None,
        }
    }

    pub(crate) fn with_span(mut self, span: proc_macro2::Span) -> Self {
        if self.location.is_none() {
            self.location = Some(SourceLocation::from_span(span));
        }
        self
    }

    /// 回傳錯誤的穩定分類。
    pub fn kind(&self) -> TranspileErrorKind {
        self.kind
    }

    /// 回傳不含分類與位置前綴的診斷訊息。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 回傳可用時的一基準原始碼位置。
    pub fn location(&self) -> Option<SourceLocation> {
        self.location
    }
}

impl fmt::Display for TranspileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.label())?;
        if let Some(location) = self.location {
            write!(f, "（第 {} 行，第 {} 欄）", location.line, location.column)?;
        }
        write!(f, "：{}", self.message)
    }
}

impl std::error::Error for TranspileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.parse_source
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

/// 將 scalar、固定陣列、整數 range、控制流程、函式與有限輸入輸出的 Rust 子集轉成 C17 原始碼。
/// 不會執行輸入原始碼或展開使用者巨集。
///
/// # Errors
/// 無效 Rust、未支援 AST／格式、語意錯誤或內嵌 NUL 會回傳錯誤。
/// [`TranspileError::kind`] 提供穩定分類，[`TranspileError::location`] 在可用時
/// 提供一基準行號與欄號。
///
/// # Examples
/// ```
/// let c = idwc::transpile(r#"fn main() { println!("Hello, World!"); }"#)?;
/// assert!(c.contains("puts(\"Hello, World!\");"));
/// # Ok::<(), idwc::TranspileError>(())
/// ```
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let ast = syn::parse_file(source).map_err(TranspileError::parse)?;
    let program = validate::lower(&ast)?;
    Ok(codegen::generate(&program))
}
