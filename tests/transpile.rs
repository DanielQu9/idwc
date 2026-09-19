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
        "fn r#main(x: i32) {}",
        "fn r#main() -> i32 { 1 }",
        r#"fn main() { #[cfg(any())] println!("hello"); }"#,
        r#"fn main() { #![allow(unused)] println!("hello"); }"#,
        r#"fn main() { std::println!("hello"); }"#,
        r#"fn main() { ::println!("hello"); }"#,
        r#"macro_rules! println { () => {} } fn main() { println!("hello"); }"#,
        "fn main() { println!(); }",
        "fn main() { println!(123); }",
        r#"fn main() { println!(b"hello"); }"#,
        r#"fn main() { println!(concat!("hello")); }"#,
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

/// 基本語法、空主體、區塊與型別推導皆應產生可編譯 IR。
#[test]
fn accepts_variables_types_and_expressions() {
    for source in [
        "fn main() {}",
        r#"fn main() { println!("a"); println!("b"); }"#,
        "fn main() { let x = 1; let y: bool = x == 1; }",
        "fn main() { let mut x: i32 = 1; x = 2; x += 1; x -= 1; x *= 2; x /= 2; x %= 2; }",
        "fn main() { let mut x = true; x = !x; }",
        "fn main() { let x = 1; { let x = x + 1; } let x = false; }",
        "fn main() { let x = 0xffi32 + 0b1010 + 0o10 + 1_000; }",
        "fn main() { let x = -2147483648i32; let y = -(2147483648); }",
        "fn main() { let x = 2; (x + 1) * 3; }",
        r#"fn main() { let r#int = 1; println!("{}", int); }"#,
        r#"fn main() { println!("{} {}", 1, true,); }"#,
        include_str!("../examples/variables.rs"),
    ] {
        assert!(transpile(source).is_ok(), "應支援：{source}");
    }
}

/// 名稱、scope、可變性與型別錯誤不可交由 C 隱式轉換或錯誤解析。
#[test]
fn rejects_semantic_errors() {
    for (source, diagnostic) in [
        ("fn main() { let x = y; }", "找不到變數"),
        ("fn main() { let x = x; }", "找不到變數"),
        ("fn main() { x = 1; }", "找不到變數"),
        ("fn main() { let x = 1; x = 2; }", "不可變"),
        ("fn main() { let x = 1; x += 2; }", "不可變"),
        ("fn main() { let mut x = 1; let x = x; x = 2; }", "不可變"),
        ("fn main() { { let x = 1; } let y = x; }", "找不到變數"),
        ("fn main() { let x: i32 = true; }", "型別不符"),
        ("fn main() { let x: bool = 1; }", "型別不符"),
        ("fn main() { let mut x = true; x = 1; }", "型別不符"),
        ("fn main() { let mut x = true; x += true; }", "型別不符"),
        ("fn main() { let x = 1 + true; }", "型別不符"),
        ("fn main() { let x = true + false; }", "型別不符"),
        ("fn main() { let x = 1 == true; }", "型別不符"),
        ("fn main() { let x = 1 && 2; }", "型別不符"),
        ("fn main() { let x = !1; }", "型別不符"),
        ("fn main() { let x = -true; }", "型別不符"),
        ("fn main() { let x = 2147483648; }", "超出 i32"),
        ("fn main() { let x = -2147483649; }", "超出 i32"),
        ("fn main() { let x = 18446744073709551615; }", "超出 i32"),
        (
            "fn main() { let x = 99999999999999999999999999999; }",
            "超出 i32",
        ),
    ] {
        let error = transpile(source).unwrap_err();
        assert!(
            matches!(error, TranspileError::Semantic(_)),
            "{source}：{error}"
        );
        assert!(error.to_string().contains(diagnostic), "{source}：{error}");
    }
}

/// v0.4.0 不擴張成完整 Rust，未實作的型別與運算式仍應拒絕。
#[test]
fn rejects_features_outside_v0_5() {
    for source in [
        "fn main() { let x; }",
        "fn main() { let x: i32; }",
        "fn main() { let _ = 1; }",
        "fn main() { let (x, y) = (1, 2); }",
        "fn main() { let ref x = 1; }",
        "fn main() { let x: i64 = 1; }",
        "fn main() { let x: &i32 = &1; }",
        "fn main() { let x: std::primitive::i32 = 1; }",
        "fn main() { let x = 1u32; }",
        "fn main() { let x = 1.0f32; }",
        "fn main() { let x = 'a'; }",
        r#"fn main() { let x = "text"; }"#,
        "fn main() { let x = [1, 2]; }",
        "fn main() { let x = 1 << 2; }",
        "fn main() { let x = 1 & 2; }",
        "fn main() { let x = 1 | 2; }",
        "fn main() { let x = 1 ^ 2; }",
        "fn main() { let x = 1 as bool; }",
        "fn main() { let x = 1.abs(); }",
        "fn main() { let x = { 1 }; }",
        "fn main() { let mut x = 1; let y = (x = 2); }",
        "fn main() { let mut x = 1; x |= 2; }",
        "fn main() { let mut x = 1; x <<= 2; }",
        "fn main() { let mut x = [1]; x[0] = 2; }",
        "fn main() { let x = 1; x }",
        "fn main() { { 1 } }",
        "fn main() { 'label: {} }",
        "fn main() { #[allow(unused)] let x = 1; }",
        "fn main() { let x = #[cfg(any())] 1; }",
        "fn main() { let x = (#[cfg(any())] 1); }",
        "fn main() { let x = 1; let y = #[cfg(any())] x; }",
        "fn main() { let x = 1; let y = x::<i32>; }",
        "fn main() { let x = module::value; }",
        "fn main() { let x = 1; let y = &x; }",
        "fn main() { let x = 1; let y = *x; }",
        "fn main() { let 中文 = 1; }",
        r#"fn main() { println!("{x}"); }"#,
        r#"fn main() { println!("{0}", 1); }"#,
        r#"fn main() { println!("{:?}", 1); }"#,
        r#"fn main() { println!("{:04}", 1); }"#,
        r#"fn main() { println!("{}", "text"); }"#,
        r#"fn main() { println!("{}", (1, 2)); }"#,
        r#"fn main() { println!("{}", 1, 2); }"#,
        r#"fn main() { let x = 1; println!("{}", x = 2); }"#,
    ] {
        assert!(
            matches!(transpile(source), Err(TranspileError::Unsupported(_))),
            "應拒絕：{source}"
        );
    }
}

/// 控制流程敘述可嵌套，unit 尾敘述可以省略分號。
#[test]
fn accepts_control_flow_statements() {
    for source in [
        "fn main() { if true {} }",
        "fn main() { if true {} else if false {} else {} }",
        "fn main() { if true { if false {} else {} } else {} }",
        "fn main() { while false {} }",
        "fn main() { while true {} }",
        "fn main() { loop {} }",
        "fn main() { loop { break } }",
        "fn main() { loop { continue } }",
        "fn main() { while true { if false { break; } else { continue; } } }",
        "fn main() { loop { { loop { break; } } continue; } }",
        include_str!("../examples/control_flow.rs"),
    ] {
        assert!(transpile(source).is_ok(), "應支援：{source}");
    }
}

/// 分支與迴圈仍須檢查 bool 條件、binding scope 與跳躍的位置。
#[test]
fn rejects_control_flow_semantic_errors() {
    for (source, diagnostic) in [
        ("fn main() { if 1 {} }", "型別不符"),
        ("fn main() { while 1 {} }", "型別不符"),
        ("fn main() { break; }", "迴圈內"),
        ("fn main() { continue; }", "迴圈內"),
        ("fn main() { if false { break; } }", "迴圈內"),
        ("fn main() { loop { break; } continue; }", "迴圈內"),
        ("fn main() { while false { break; } break; }", "迴圈內"),
        (
            "fn main() { if true { let x = 1; } let y = x; }",
            "找不到變數",
        ),
        (
            "fn main() { if true { let x = 1; } else { let y = x; } }",
            "找不到變數",
        ),
        (
            "fn main() { if true { let x = 1; } else if x == 1 {} }",
            "找不到變數",
        ),
        (
            "fn main() { while false { let x = 1; } let y = x; }",
            "找不到變數",
        ),
        (
            "fn main() { loop { let x = 1; break; } let y = x; }",
            "找不到變數",
        ),
        ("fn main() { while x == 1 { let x = 1; } }", "找不到變數"),
        ("fn main() { let x = 1; if false { x = 2; } }", "不可變"),
        (
            "fn main() { let mut x = 1; while false { x = true; } }",
            "型別不符",
        ),
        (
            "fn main() { loop { break; } if true { continue; } }",
            "迴圈內",
        ),
    ] {
        let error = transpile(source).unwrap_err();
        assert!(
            matches!(error, TranspileError::Semantic(_)),
            "{source}：{error}"
        );
        assert!(error.to_string().contains(diagnostic), "{source}：{error}");
    }
}

/// 帶值／標籤的控制流程與不支援的 AST 必須明確拒絕，即使不會執行。
#[test]
fn rejects_unsupported_control_flow_forms() {
    for source in [
        "fn main() { let x = if true { 1 } else { 2 }; }",
        "fn main() { let x = loop { break 1; }; }",
        "fn main() { let x = while false {}; }",
        "fn main() { if true { 1 } else { 2 }; }",
        "fn main() { loop { break 1; } }",
        "fn main() { while true { break 1; } }",
        "fn main() { 'outer: loop { break; } }",
        "fn main() { 'outer: while true { break; } }",
        "fn main() { loop { break 'outer; } }",
        "fn main() { loop { continue 'outer; } }",
        "fn main() { if let true = true {} }",
        "fn main() { while let true = true {} }",
        "fn main() { while { true } {} }",
        "fn main() { for x in 0..3 {} }",
        "fn main() { match 1 { _ => {} } }",
        "fn main() { #[cfg(any())] if true {} }",
        "fn main() { #[cfg(any())] while true {} }",
        "fn main() { #[cfg(any())] loop {} }",
        "fn main() { loop { #[cfg(any())] break; } }",
        "fn main() { loop { #[cfg(any())] continue; } }",
        "fn main() { while false { #![allow(unused)] } }",
        "fn main() { loop { #![allow(unused)] break; } }",
        "fn main() { if false { let x = 1.0f32; } }",
    ] {
        let result = transpile(source);
        assert!(
            matches!(result, Err(TranspileError::Unsupported(_))),
            "應拒絕：{source}，結果：{result:?}"
        );
    }
}

/// Rust 語法解析失敗應與不支援的語法區分。
#[test]
fn reports_parse_errors() {
    for source in [
        "fn main( {",
        "fn main() {",
        "fn main() { let = ; }",
        // syn 的 Block 不接受 if 分支內的 inner attribute，因此屬於解析錯誤。
        "fn main() { if true { #![allow(unused)] } }",
        "fn main() { if true {} else { #![allow(unused)] } }",
    ] {
        let error = transpile(source).unwrap_err();
        assert!(matches!(error, TranspileError::Parse(_)));
        assert!(error.to_string().contains("Rust 解析失敗"));
        assert!(std::error::Error::source(&error).is_some());
    }
}

/// 簽章先註冊，函式各自保留 scope 與回傳型別。
#[test]
fn accepts_functions_return_and_io() {
    for source in [
        include_str!("../examples/functions_stdin.rs"),
        "fn main() { return; } fn unused() {}",
        "fn r#main() {}",
        "fn main() { return (); } fn unit() -> () { () }",
        "fn main() { unit() } fn unit() { return (); }",
        "fn main() { f(1); } fn f(mut x: i32) -> i32 { x += 1; x }",
        "fn main() {} fn f(x: bool) -> i32 { if x { return 1; } else { return 2; } }",
        "fn main() {} fn f() -> i32 { { return 1; } }",
        "fn main() { idwc::io::flush_stdout() }",
        "fn main() { let n = idwc::io::read_i32(); while n > 0 { return; } }",
    ] {
        let result = transpile(source);
        assert!(result.is_ok(), "{source}：{result:?}");
    }
}

/// 不讓 C 的隱式轉型、宣告順序或缺少 return 隱藏錯誤。
#[test]
fn rejects_function_semantic_errors() {
    for source in [
        "",
        "fn other() {}",
        "fn main() {} fn main() {}",
        "fn main() {} fn f() {} fn f() {}",
        "fn main() {} fn f(x: i32, x: i32) {}",
        "fn main() { f(); }",
        "fn main() { f(); } fn f(x: i32) {}",
        "fn main() { f(1, 2); } fn f(x: i32) {}",
        "fn main() { f(true); } fn f(x: i32) {}",
        "fn main() { let f = 1; f(); } fn f() {}",
        "fn main() { return 1; }",
        "fn main() {} fn f() -> i32 { return; }",
        "fn main() {} fn f() -> i32 { true }",
        "fn main() {} fn f() -> i32 {}",
        "fn main() {} fn f(x: bool) -> i32 { if x { return 1; } }",
        "fn main() {} fn f() -> i32 { while true { return 1; } }",
        "fn main() {} fn f() -> i32 { 1; }",
        "fn main() {} fn f(x: i32) { x = 2; }",
        "fn main() { let x = 1; f(); } fn f() { println!(\"{}\", x); }",
        "fn main() { idwc::io::read_i32(1); }",
        "fn main() { idwc::io::flush_stdout(1); }",
    ] {
        let result = transpile(source);
        assert!(
            matches!(result, Err(TranspileError::Semantic(_))),
            "{source}：{result:?}"
        );
    }
}

#[test]
fn rejects_unsupported_function_and_io_forms() {
    for source in [
        "fn main() { main(); }",
        "fn main() {} pub fn f() {}",
        "fn main() {} fn f<T>() {}",
        "fn main() {} fn f(x: &i32) {}",
        "fn main() {} fn f((x, y): (i32, i32)) {}",
        "fn main() {} fn f(x: ()) {}",
        "fn main() {} fn f() -> f32 { 1.0 }",
        "fn main() {} fn f() -> i32 { if true { 1 } else { 2 } }",
        "fn main() { let x = idwc::io::flush_stdout(); }",
        "fn main() { idwc::io::read_f32(); }",
        "fn main() { std::io::stdin(); }",
        "fn main() { idwc::io::read_i32::<i32>(); }",
        "fn main() { println!(\"{}\", ()); }",
        "fn main() { let x = () == (); }",
        "fn main() { fn nested() {} }",
    ] {
        let result = transpile(source);
        assert!(
            matches!(result, Err(TranspileError::Unsupported(_))),
            "{source}：{result:?}"
        );
    }
}

/// f64 字面量、運算、傳值及精度格式皆經過型別檢查。
#[test]
fn accepts_floating_point_subset() {
    for source in [
        include_str!("../examples/floating_point.rs"),
        "fn main() { let x = 1f64; let y = -1f64; let z = 1_2.5_0e-1f64; }",
        "fn main() { let mut x: f64 = 1.0; x += 2.0; x -= 1.0; x *= 3.0; x /= 2.0; x = -x; }",
        "fn main() { let x = 1e-9999; println!(\"{} {:.0} {:.18}\", x, x, x); }",
        "fn main() { let x = idwc::io::read_f64(); println!(\"{}\", echo(x)); } fn echo(x: f64) -> f64 { x }",
    ] {
        let result = transpile(source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
}

#[test]
fn rejects_floating_point_type_errors() {
    for source in [
        "fn main() { let x: i32 = 1.0; }",
        "fn main() { let x: f64 = 1; }",
        "fn main() { let x = 1.0 + 1; }",
        "fn main() { let x = 1 + 1.0; }",
        "fn main() { let x = 1.0 == 1; }",
        "fn main() { let x = !1.0; }",
        "fn main() { if 1.0 {} }",
        "fn main() { let mut x = 1.0; x += 1; }",
        "fn main() { let mut x = 1; x = 1.0; }",
        "fn main() { f(1); } fn f(x: f64) {}",
        "fn main() {} fn f() -> f64 { 1 }",
        "fn main() { println!(\"{:.2}\", 1); }",
        "fn main() { println!(\"{:.2}\", true); }",
        "fn main() { let x = 1e9999; }",
        "fn main() { idwc::io::read_f64(1); }",
    ] {
        let result = transpile(source);
        assert!(
            matches!(result, Err(TranspileError::Semantic(_))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn rejects_unsupported_floating_point_forms() {
    for source in [
        "fn main() { let x = 1f32; }",
        "fn main() { let x = 0b1f64; }",
        "fn main() { let x = 1.0 % 2.0; }",
        "fn main() { let mut x = 1.0; x %= 2.0; }",
        "fn main() { let x = 1 as f64; }",
        "fn main() { let x = 1.0.powi(2); }",
        "fn main() { println!(\"{:.19}\", 1.0); }",
        "fn main() { println!(\"{:.}\", 1.0); }",
        "fn main() { println!(\"{:.2e}\", 1.0); }",
        "fn main() { println!(\"{:.precision$}\", 1.0); }",
    ] {
        let result = transpile(source);
        assert!(
            matches!(result, Err(TranspileError::Unsupported(_))),
            "{source}: {result:?}"
        );
    }
}
