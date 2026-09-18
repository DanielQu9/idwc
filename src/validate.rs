use syn::{Item, ReturnType, Visibility, ext::IdentExt};

use crate::{TranspileError, ir::Program};

/// 白名單驗證整份 AST，僅將已支援的程式降低為 IR。
pub(crate) fn lower(file: &syn::File) -> Result<Program, TranspileError> {
    if !file.attrs.is_empty() || file.shebang.is_some() {
        return Err(TranspileError::Unsupported("不接受檔案屬性或 shebang"));
    }
    let mut functions = Vec::new();
    for item in &file.items {
        let Item::Fn(function) = item else {
            return Err(TranspileError::Unsupported("頂層僅接受函式"));
        };
        validate_signature(function)?;
        functions.push(function);
    }
    crate::semantic::analyze(&functions)
}

/// 驗證函式共通限制；main 仍須無參數與無顯式回傳型別。
fn validate_signature(function: &syn::ItemFn) -> Result<(), TranspileError> {
    let signature = &function.sig;
    if !function.attrs.is_empty()
        || !matches!(function.vis, Visibility::Inherited)
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.generics.lt_token.is_some()
        || signature.generics.gt_token.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.variadic.is_some()
    {
        return Err(TranspileError::Unsupported(
            "函式必須無屬性、修飾詞、泛型與可變參數，且不可指定可見性",
        ));
    }
    if signature.ident.unraw() == "main"
        && (!signature.inputs.is_empty() || !matches!(signature.output, ReturnType::Default))
    {
        return Err(TranspileError::Unsupported(
            "main 必須無參數且無顯式回傳型別",
        ));
    }
    Ok(())
}
