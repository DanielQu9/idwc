use syn::parse::Parser;
use syn::{Expr, Item, LitStr, ReturnType, Stmt, Token, Visibility};

use crate::{TranspileError, ir::Program};

/// 白名單驗證整份 AST，僅將已支援的程式降低為 IR。
pub(crate) fn lower(file: &syn::File) -> Result<Program, TranspileError> {
    if !file.attrs.is_empty() || file.shebang.is_some() {
        return Err(TranspileError::Unsupported("不接受檔案屬性或 shebang"));
    }
    let [Item::Fn(function)] = file.items.as_slice() else {
        return Err(TranspileError::Unsupported("僅接受一個 fn main()"));
    };
    let signature = &function.sig;
    if !function.attrs.is_empty()
        || !matches!(function.vis, Visibility::Inherited)
        || signature.ident != "main"
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.generics.lt_token.is_some()
        || signature.generics.gt_token.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || !signature.inputs.is_empty()
        || signature.variadic.is_some()
        || !matches!(signature.output, ReturnType::Default)
    {
        return Err(TranspileError::Unsupported(
            "main 必須是無屬性、無修飾詞、無泛型、無參數且無顯式回傳型別的 fn main()",
        ));
    }
    let mac = match function.block.stmts.as_slice() {
        [Stmt::Macro(statement)] if statement.attrs.is_empty() => &statement.mac,
        [Stmt::Expr(Expr::Macro(expression), _)] if expression.attrs.is_empty() => &expression.mac,
        _ => {
            return Err(TranspileError::Unsupported(
                "main 主體僅接受一個 println!，不接受其他敘述或屬性",
            ));
        }
    };
    if !mac.path.is_ident("println") {
        return Err(TranspileError::Unsupported("僅接受未限定路徑的 println!"));
    }

    // 只解析字面量與可選的尾逗號，絕不展開巨集。
    let parser = |input: syn::parse::ParseStream<'_>| {
        let literal: LitStr = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(literal)
    };
    let literal = parser.parse2(mac.tokens.clone()).map_err(|_| {
        TranspileError::Unsupported("println! 僅接受一個字串字面量，不接受格式參數")
    })?;
    if !literal.suffix().is_empty() {
        return Err(TranspileError::Unsupported("字串字面量不接受後綴"));
    }
    let message = decode_format(&literal.value())?;
    if message.contains('\0') {
        // puts 會在 NUL 截斷；拒絕輸入，避免產生語意不符的 C。
        return Err(TranspileError::Unsupported("字串不接受內嵌 NUL 字元"));
    }
    Ok(Program { message })
}

/// 將 {{ 與 }} 還原；拒絕所有需要格式參數或不合法的大括號。
fn decode_format(value: &str) -> Result<String, TranspileError> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if matches!(ch, '{' | '}') && chars.next() != Some(ch) {
            return Err(TranspileError::Unsupported(
                "不接受格式佔位符；文字大括號請使用 {{ 或 }}",
            ));
        }
        result.push(ch);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::decode_format;

    /// 格式大括號應逐對解碼，拒絕多餘或不完整的大括號。
    #[test]
    fn decodes_only_paired_literal_braces() {
        assert_eq!(decode_format("{{中文}} {{{{}}}}").unwrap(), "{中文} {{}}");
        for value in ["{", "}", "{{{", "}}}", "{{name}", "{0}", "{:}"] {
            assert!(decode_format(value).is_err(), "應拒絕：{value}");
        }
    }
}
