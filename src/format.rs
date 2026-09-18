use syn::{Expr, LitStr, Token, parse::Parser};

use crate::{TranspileError, ir::PrintPart};

/// 解析 print!／println! 的字串與參數，不展開或執行巨集。
pub(crate) fn parse(mac: &syn::Macro) -> Result<(Vec<PrintPart>, Vec<Expr>), TranspileError> {
    if !mac.path.is_ident("println") && !mac.path.is_ident("print") {
        return Err(TranspileError::Unsupported(
            "僅接受未限定路徑的 print!／println!",
        ));
    }
    let parser = |input: syn::parse::ParseStream<'_>| {
        let literal: LitStr = input.parse()?;
        let mut arguments = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            arguments.push(input.parse::<Expr>()?);
        }
        Ok((literal, arguments))
    };
    let (literal, arguments) = parser.parse2(mac.tokens.clone()).map_err(|_| {
        TranspileError::Unsupported("print!／println! 必須以字串字面量開頭，參數以逗號分隔")
    })?;
    if !literal.suffix().is_empty() {
        return Err(TranspileError::Unsupported("字串字面量不接受後綴"));
    }
    let parts = decode(&literal.value(), arguments.len())?;
    Ok((parts, arguments))
}

/// 還原 {{／}} 與依序的 {}，拒絕其他格式語法並檢查參數數量。
fn decode(value: &str, argument_count: usize) -> Result<Vec<PrintPart>, TranspileError> {
    if value.contains('\0') {
        return Err(TranspileError::Unsupported("字串不接受內嵌 NUL 字元"));
    }
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut next_argument = 0;
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '{' => match chars.next() {
                Some('{') => text.push('{'),
                Some('}') => {
                    parts.push(PrintPart::Text(std::mem::take(&mut text)));
                    parts.push(PrintPart::Argument(next_argument));
                    next_argument += 1;
                }
                _ => {
                    return Err(TranspileError::Unsupported(
                        "僅接受 {} 佔位符與 {{／}} 文字大括號",
                    ));
                }
            },
            '}' if chars.next() == Some('}') => text.push('}'),
            '}' => return Err(TranspileError::Unsupported("未配對的 }；請使用 }}")),
            _ => text.push(ch),
        }
    }
    parts.push(PrintPart::Text(text));
    if next_argument != argument_count {
        return Err(TranspileError::Unsupported("{} 佔位符與參數數量必須相同"));
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::{PrintPart, decode};

    /// 格式大括號應逐對解碼，拒絕多餘或不完整的大括號。
    #[test]
    fn decodes_only_paired_literal_braces() {
        let parts = decode("{{中文}} {{{{}}}}", 0).unwrap();
        assert!(matches!(&parts[0], PrintPart::Text(text) if text == "{中文} {{}}"));
        for value in ["{", "}", "{{{", "}}}", "{{name}", "{0}", "{:}"] {
            assert!(decode(value, 0).is_err(), "應拒絕：{value}");
        }
    }

    /// 跳脫的大括號不能消耗參數；每個 {} 恰好消耗一個參數。
    #[test]
    fn checks_placeholder_counts() {
        assert!(decode("{{{}}} {}", 2).is_ok());
        assert!(decode("{{}}", 1).is_err());
        assert!(decode("{}", 0).is_err());
        assert!(decode("{}", 2).is_err());
    }
}
