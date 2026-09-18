use idwc::{TranspileError, transpile};

/// MVP 的 Hello World 應生成可讀且完整的 C 程式。
#[test]
fn hello_world_generates_expected_c() {
    let generated = transpile(include_str!("../examples/hello.rs")).unwrap();
    assert_eq!(
        generated,
        "#include <stdio.h>\n\nint main(void) {\n    puts(\"Hello, World!\");\n    return 0;\n}\n"
    );
}

/// 已支援的 println! 形式應保持一致輸出。
#[test]
fn accepts_supported_macro_forms() {
    for source in [
        r#"fn main() { println!("hello"); }"#,
        r#"fn main() { println!("hello") }"#,
        r#"fn main() { println!("hello",); }"#,
        r#"fn main() { println!["hello"]; }"#,
        r#"fn main() { println!{"hello"} }"#,
        "// comment\nfn main() { /* comment */ println!(\"hello\"); }",
    ] {
        assert!(transpile(source).unwrap().contains("puts(\"hello\");"));
    }
}

/// AST 白名單應拒絕所有未支援結構，包含條件編譯和自訂巨集。
#[test]
fn rejects_unsupported_syntax() {
    let sources = [
        "",
        r#"fn other() { println!("hello"); }"#,
        r#"fn main() { println!("hello"); } fn other() {}"#,
        r#"fn main() { println!("hello"); } fn main() {}"#,
        r#"const X: i32 = 1; fn main() { println!("hello"); }"#,
        r#"use std::io; fn main() { println!("hello"); }"#,
        r#"mod nested { fn main() { println!("hello"); } }"#,
        r#"#![allow(unused)] fn main() { println!("hello"); }"#,
        "#!/usr/bin/env rust\nfn main() { println!(\"hello\"); }",
        r#"#[cfg(any())] fn main() { println!("hello"); }"#,
        r#"/// main docs
fn main() { println!("hello"); }"#,
        r#"pub fn main() { println!("hello"); }"#,
        r#"pub(crate) fn main() { println!("hello"); }"#,
        r#"async fn main() { println!("hello"); }"#,
        r#"unsafe fn main() { println!("hello"); }"#,
        r#"const fn main() { println!("hello"); }"#,
        r#"extern "C" fn main() { println!("hello"); }"#,
        r#"fn main<T>() { println!("hello"); }"#,
        r#"fn main<>() { println!("hello"); }"#,
        r#"fn main() where i32: Copy { println!("hello"); }"#,
        r#"fn main(x: i32) { println!("hello"); }"#,
        r#"fn main() -> () { println!("hello"); }"#,
        "fn main() {}",
        r#"fn main() { println!("a"); println!("b"); }"#,
        r#"fn main() { let x = 1; println!("hello"); }"#,
        r#"fn main() { println!("hello"); return; }"#,
        r#"fn main() { { println!("hello"); } }"#,
        r#"fn main() { if true { println!("hello"); } }"#,
        r#"fn main() { #[cfg(any())] println!("hello"); }"#,
        r#"fn main() { #![allow(unused)] println!("hello"); }"#,
        r#"fn main() { print!("hello"); }"#,
        r#"fn main() { std::println!("hello"); }"#,
        r#"fn main() { ::println!("hello"); }"#,
        r#"macro_rules! println { () => {} } fn main() { println!("hello"); }"#,
        "fn main() { println!(); }",
        "fn main() { println!(123); }",
        r#"fn main() { println!(b"hello"); }"#,
        r#"fn main() { println!(concat!("hello")); }"#,
        r#"fn main() { println!("{}", 1); }"#,
        r#"fn main() { println!("hello", "other"); }"#,
        r#"fn main() { println!("hello",,); }"#,
        r#"fn main() { println!("hello"suffix); }"#,
        r#"fn main() { println!("{}"); }"#,
        r#"fn main() { println!("{name}"); }"#,
        r#"fn main() { println!("{:?}"); }"#,
        r#"fn main() { println!("{"); }"#,
        r#"fn main() { println!("}"); }"#,
        r#"fn main() { println!("{{}"); }"#,
        r#"fn main() { println!("before\0after"); }"#,
    ];
    for source in sources {
        assert!(
            matches!(transpile(source), Err(TranspileError::Unsupported(_))),
            "應拒絕未支援的原始碼：{source}"
        );
    }
}

/// Rust 語法解析失敗應與不支援的語法區分。
#[test]
fn reports_parse_errors() {
    for source in ["fn main( {", "fn main() {", "fn main() { let = ; }"] {
        let error = transpile(source).unwrap_err();
        assert!(matches!(error, TranspileError::Parse(_)));
        assert!(error.to_string().contains("Rust 解析失敗"));
        assert!(std::error::Error::source(&error).is_some());
    }
}
