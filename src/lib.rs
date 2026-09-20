//! 將經過白名單驗證的 Rust 子集轉成獨立的 C17 程式。
//! v1.2.0 提供穩定的嚴格模式、可選的可讀 C 模式與 bounded collection
//! 容量設定；預設入口為 [`transpile`]。

mod codegen;
mod format;
pub mod io;
mod ir;
mod semantic;
mod stupid_codegen;
mod validate;

use std::fmt;

/// Selects the C generation contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranspileMode {
    /// Preserves the documented Rust-subset semantics and runtime checks.
    #[default]
    Strict,
    /// Prioritizes readable C and source names while omitting strict runtime checks.
    Stupid,
}

/// Options for [`transpile_with_options`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TranspileOptions {
    mode: TranspileMode,
    string_capacity: usize,
    vec_capacity: usize,
}

impl TranspileOptions {
    /// Creates options for the selected translation mode.
    pub const fn new(mode: TranspileMode) -> Self {
        Self {
            mode,
            string_capacity: DEFAULT_STRING_CAPACITY,
            vec_capacity: DEFAULT_VEC_CAPACITY,
        }
    }

    /// Returns the selected translation mode.
    pub const fn mode(self) -> TranspileMode {
        self.mode
    }

    /// Sets the generated bounded String payload capacity in bytes.
    pub const fn with_string_capacity(mut self, capacity: usize) -> Self {
        self.string_capacity = capacity;
        self
    }

    /// Sets the generated fixed-capacity Vec limit in elements.
    pub const fn with_vec_capacity(mut self, capacity: usize) -> Self {
        self.vec_capacity = capacity;
        self
    }

    /// Returns the generated bounded String payload capacity in bytes.
    pub const fn string_capacity(self) -> usize {
        self.string_capacity
    }

    /// Returns the generated fixed-capacity Vec limit in elements.
    pub const fn vec_capacity(self) -> usize {
        self.vec_capacity
    }

    fn validate(self) -> Result<(), TranspileError> {
        for (name, value) in [
            ("String capacity", self.string_capacity),
            ("Vec capacity", self.vec_capacity),
        ] {
            if !(1..=MAX_COLLECTION_CAPACITY).contains(&value) {
                return Err(TranspileError::configuration(format!(
                    "{name} 必須介於 1 與 {MAX_COLLECTION_CAPACITY} 之間"
                )));
            }
        }
        Ok(())
    }
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self::new(TranspileMode::Strict)
    }
}

/// Default bounded String payload capacity in bytes.
pub const DEFAULT_STRING_CAPACITY: usize = 4096;

/// Default fixed-capacity Vec limit in elements.
pub const DEFAULT_VEC_CAPACITY: usize = 2048;

/// Largest String or Vec capacity accepted by translation options.
pub const MAX_COLLECTION_CAPACITY: usize = 65_536;

/// 轉譯失敗的穩定分類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranspileErrorKind {
    /// Translation options are outside their documented bounds.
    Configuration,
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
            Self::Configuration => "設定錯誤",
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
    fn configuration(message: impl Into<String>) -> Self {
        Self {
            kind: TranspileErrorKind::Configuration,
            message: message.into(),
            location: None,
            parse_source: None,
        }
    }

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

/// 將 scalar、固定陣列、bounded String／Vec、控制流程、函式與有限輸入輸出的 Rust 子集轉成 C17 原始碼。
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
    transpile_with_options(source, TranspileOptions::default())
}

/// Translates the supported Rust subset to C17 using an explicit generation mode.
/// Parsing, whitelist validation, name resolution, and type checking apply in every mode.
/// [`TranspileMode::Stupid`] deliberately relaxes runtime semantic guarantees.
///
/// # Errors
/// Returns [`TranspileError`] for invalid Rust, unsupported syntax, or semantic errors.
///
/// # Examples
/// ```
/// use idwc::{TranspileMode, TranspileOptions, transpile_with_options};
///
/// let c = transpile_with_options(
///     r#"fn main() { let answer = 42; println!("{}", answer); }"#,
///     TranspileOptions::new(TranspileMode::Stupid),
/// )?;
/// assert!(c.contains("const int answer = 42;"));
/// # Ok::<(), idwc::TranspileError>(())
/// ```
pub fn transpile_with_options(
    source: &str,
    options: TranspileOptions,
) -> Result<String, TranspileError> {
    options.validate()?;
    let ast = syn::parse_file(source).map_err(TranspileError::parse)?;
    let program = validate::lower(&ast)?;
    Ok(match options.mode() {
        TranspileMode::Strict => codegen::generate(&program, options),
        TranspileMode::Stupid => stupid_codegen::generate(&program, options),
    })
}
