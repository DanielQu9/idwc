use syn::{Item, ReturnType, Visibility};

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
    crate::semantic::analyze(&function.block)
}
