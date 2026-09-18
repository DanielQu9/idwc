use std::collections::HashMap;

use syn::{BinOp, Expr, Lit, Pat, Stmt, UnOp, ext::IdentExt};

use crate::{
    TranspileError,
    ir::{BinaryOp, Expression, ExpressionKind, Program, Statement, Type, UnaryOp},
};

/// 名稱解析後的 binding；id 在離開 scope 後也不重用。
#[derive(Clone, Copy)]
struct Binding {
    id: usize,
    ty: Type,
    mutable: bool,
}

/// 以 scope 堆疊解析名稱，並在降低 AST 時檢查型別與可變性。
struct Analyzer {
    scopes: Vec<HashMap<String, Binding>>,
    next_id: usize,
    loop_depth: usize,
}

/// 分析 main 主體，產生不再依賴 Rust 名稱解析的 IR。
pub(crate) fn analyze(block: &syn::Block) -> Result<Program, TranspileError> {
    let mut analyzer = Analyzer {
        scopes: Vec::new(),
        next_id: 0,
        loop_depth: 0,
    };
    Ok(Program {
        statements: analyzer.block(block)?,
    })
}

impl Analyzer {
    /// 每個區塊建立 scope；離開後恢復外層 binding。
    fn block(&mut self, block: &syn::Block) -> Result<Vec<Statement>, TranspileError> {
        self.scopes.push(HashMap::new());
        let result = block
            .stmts
            .iter()
            .map(|statement| self.statement(statement))
            .collect();
        self.scopes.pop();
        result
    }

    /// 敘述採白名單；有值的尾運算式不能作為 main 或敘述區塊的回傳值。
    fn statement(&mut self, statement: &Stmt) -> Result<Statement, TranspileError> {
        match statement {
            Stmt::Local(local) if local.attrs.is_empty() => self.local(local),
            Stmt::Macro(mac) if mac.attrs.is_empty() => self.print(&mac.mac),
            Stmt::Expr(Expr::Macro(mac), _) if mac.attrs.is_empty() => self.print(&mac.mac),
            Stmt::Expr(Expr::Block(block), _)
                if block.attrs.is_empty() && block.label.is_none() =>
            {
                Ok(Statement::Block(self.block(&block.block)?))
            }
            Stmt::Expr(Expr::If(branch), _) if branch.attrs.is_empty() => self.branch(branch),
            Stmt::Expr(Expr::While(while_loop), _)
                if while_loop.attrs.is_empty() && while_loop.label.is_none() =>
            {
                let condition = self.condition(&while_loop.cond)?;
                let body = self.loop_body(&while_loop.body)?;
                Ok(Statement::While { condition, body })
            }
            Stmt::Expr(Expr::Loop(loop_expr), _)
                if loop_expr.attrs.is_empty() && loop_expr.label.is_none() =>
            {
                Ok(Statement::Loop(self.loop_body(&loop_expr.body)?))
            }
            Stmt::Expr(Expr::Break(jump), _)
                if jump.attrs.is_empty() && jump.label.is_none() && jump.expr.is_none() =>
            {
                self.check_loop_context()?;
                Ok(Statement::Break)
            }
            Stmt::Expr(Expr::Continue(jump), _)
                if jump.attrs.is_empty() && jump.label.is_none() =>
            {
                self.check_loop_context()?;
                Ok(Statement::Continue)
            }
            Stmt::Expr(Expr::Assign(assign), _) if assign.attrs.is_empty() => {
                let binding = self.assignment_target(&assign.left)?;
                let value = self.expression(&assign.right)?;
                same_type(binding.ty, value.ty)?;
                Ok(Statement::Assign {
                    id: binding.id,
                    value,
                })
            }
            Stmt::Expr(Expr::Binary(binary), _) if binary.attrs.is_empty() => {
                let op = match binary.op {
                    BinOp::AddAssign(_) => Some(BinaryOp::Add),
                    BinOp::SubAssign(_) => Some(BinaryOp::Subtract),
                    BinOp::MulAssign(_) => Some(BinaryOp::Multiply),
                    BinOp::DivAssign(_) => Some(BinaryOp::Divide),
                    BinOp::RemAssign(_) => Some(BinaryOp::Remainder),
                    _ => None,
                };
                if let Some(op) = op {
                    let binding = self.assignment_target(&binary.left)?;
                    let right = self.expression(&binary.right)?;
                    let left = Expression {
                        ty: binding.ty,
                        kind: ExpressionKind::Variable(binding.id),
                    };
                    let value = binary_expression(op, left, right)?;
                    return Ok(Statement::Assign {
                        id: binding.id,
                        value,
                    });
                }
                self.evaluate_statement(statement)
            }
            Stmt::Expr(_, _) => self.evaluate_statement(statement),
            _ => Err(TranspileError::Unsupported(
                "不接受此敘述、item、pattern 或屬性",
            )),
        }
    }

    /// 條件只接受純 bool 運算式，不使用 C 的整數 truthiness。
    fn condition(&self, expr: &Expr) -> Result<Expression, TranspileError> {
        let condition = self.expression(expr)?;
        same_type(Type::Bool, condition.ty)?;
        Ok(condition)
    }

    /// 分別分析分支 scope；else if 延後到 else 路徑才求值。
    fn branch(&mut self, branch: &syn::ExprIf) -> Result<Statement, TranspileError> {
        let condition = self.condition(&branch.cond)?;
        let then_branch = self.block(&branch.then_branch)?;
        let else_branch = match &branch.else_branch {
            None => None,
            Some((_, expr)) => match expr.as_ref() {
                Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
                    Some(self.block(&block.block)?)
                }
                Expr::If(branch) if branch.attrs.is_empty() => Some(vec![self.branch(branch)?]),
                _ => {
                    return Err(TranspileError::Unsupported(
                        "else 僅接受無屬性的區塊或 else if",
                    ));
                }
            },
        };
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// 在迴圈主體內允許跳躍；無論分析成功或失敗都恢復外層深度。
    fn loop_body(&mut self, block: &syn::Block) -> Result<Vec<Statement>, TranspileError> {
        self.loop_depth += 1;
        let result = self.block(block);
        self.loop_depth -= 1;
        result
    }

    /// 拒絕迴圈之外的 break／continue，包含不可到達的分支。
    fn check_loop_context(&self) -> Result<(), TranspileError> {
        if self.loop_depth == 0 {
            return Err(TranspileError::Semantic(
                "break／continue 僅允許在迴圈內".into(),
            ));
        }
        Ok(())
    }

    /// 只有帶分號的純值運算式可捨棄結果。
    fn evaluate_statement(&self, statement: &Stmt) -> Result<Statement, TranspileError> {
        if let Stmt::Expr(expr, Some(_)) = statement {
            return Ok(Statement::Evaluate(self.expression(expr)?));
        }
        Err(TranspileError::Unsupported(
            "區塊尾端僅接受 unit 敘述；純值運算式必須帶分號",
        ))
    }

    /// 先分析 initializer，再註冊 binding，保留 Rust shadowing 的可見性。
    fn local(&mut self, local: &syn::Local) -> Result<Statement, TranspileError> {
        let (pattern, annotation) = match &local.pat {
            Pat::Type(typed) if typed.attrs.is_empty() => {
                (typed.pat.as_ref(), Some(parse_type(&typed.ty)?))
            }
            pattern => (pattern, None),
        };
        let Pat::Ident(ident) = pattern else {
            return Err(TranspileError::Unsupported("let 僅接受單一識別字 binding"));
        };
        if !ident.attrs.is_empty() || ident.by_ref.is_some() || ident.subpat.is_some() {
            return Err(TranspileError::Unsupported(
                "不接受 binding 屬性、ref 或子 pattern",
            ));
        }
        if !ident.ident.unraw().to_string().is_ascii() {
            return Err(TranspileError::Unsupported(
                "識別字暫僅接受 ASCII；尚未實作 Rust 的 Unicode 正規化",
            ));
        }
        let init = local
            .init
            .as_ref()
            .ok_or(TranspileError::Unsupported("let 必須在宣告時初始化"))?;
        if init.diverge.is_some() {
            return Err(TranspileError::Unsupported("不接受 let-else"));
        }
        let value = self.expression(&init.expr)?;
        if let Some(annotation) = annotation {
            same_type(annotation, value.ty)?;
        }
        let binding = Binding {
            id: self.next_id,
            ty: value.ty,
            mutable: ident.mutability.is_some(),
        };
        self.next_id += 1;
        // local 僅由 block 呼叫；此時堆疊必定包含目前區塊。
        let scope_index = self.scopes.len() - 1;
        self.scopes[scope_index].insert(ident.ident.unraw().to_string(), binding);
        Ok(Statement::Let {
            id: binding.id,
            mutable: binding.mutable,
            value,
        })
    }

    /// println! 參數只接受型別已知的純 i32／bool 運算式。
    fn print(&self, mac: &syn::Macro) -> Result<Statement, TranspileError> {
        let (parts, arguments) = crate::format::parse(mac)?;
        let arguments = arguments
            .iter()
            .map(|expr| self.expression(expr))
            .collect::<Result<_, _>>()?;
        Ok(Statement::Print { parts, arguments })
    }

    /// 從內向外查詢識別字；不接受限定路徑或泛型參數。
    fn resolve(&self, expr: &Expr) -> Result<Binding, TranspileError> {
        let Expr::Path(path) = expr else {
            return Err(TranspileError::Unsupported("變數必須是單一識別字"));
        };
        if !path.attrs.is_empty() || path.qself.is_some() {
            return Err(TranspileError::Unsupported("不接受路徑屬性或限定路徑"));
        }
        let ident = path
            .path
            .get_ident()
            .ok_or(TranspileError::Unsupported("變數不接受限定路徑或泛型參數"))?;
        let name = ident.unraw().to_string();
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).copied())
            .ok_or_else(|| TranspileError::Semantic(format!("找不到變數 `{name}`")))
    }

    /// 賦值只能寫入先前宣告的可變 binding。
    fn assignment_target(&self, expr: &Expr) -> Result<Binding, TranspileError> {
        let binding = self.resolve(expr)?;
        if !binding.mutable {
            return Err(TranspileError::Semantic("不能賦值給不可變 binding".into()));
        }
        Ok(binding)
    }

    /// 遞迴檢查純值運算式；所有未列出的 AST 與屬性皆拒絕。
    fn expression(&self, expr: &Expr) -> Result<Expression, TranspileError> {
        match expr {
            Expr::Lit(literal) if literal.attrs.is_empty() => match &literal.lit {
                Lit::Int(integer) => Ok(Expression {
                    ty: Type::I32,
                    kind: ExpressionKind::Integer(integer_value(integer, false)?),
                }),
                Lit::Bool(boolean) => Ok(Expression {
                    ty: Type::Bool,
                    kind: ExpressionKind::Boolean(boolean.value),
                }),
                _ => Err(TranspileError::Unsupported("值僅接受 i32 與 bool 字面量")),
            },
            Expr::Path(_) => {
                let binding = self.resolve(expr)?;
                Ok(Expression {
                    ty: binding.ty,
                    kind: ExpressionKind::Variable(binding.id),
                })
            }
            Expr::Paren(paren) if paren.attrs.is_empty() => self.expression(&paren.expr),
            Expr::Unary(unary) if unary.attrs.is_empty() => {
                let (op, ty) = match unary.op {
                    UnOp::Neg(_) => (UnaryOp::Negate, Type::I32),
                    UnOp::Not(_) => (UnaryOp::Not, Type::Bool),
                    _ => return Err(TranspileError::Unsupported("不接受此一元運算")),
                };
                // Rust 允許 -2147483648（也允許括號），但正的 2147483648 不是 i32。
                if matches!(op, UnaryOp::Negate)
                    && let Some(integer) = literal_integer(&unary.expr)
                {
                    return Ok(Expression {
                        ty,
                        kind: ExpressionKind::Integer(integer_value(integer, true)?),
                    });
                }
                let operand = self.expression(&unary.expr)?;
                same_type(ty, operand.ty)?;
                Ok(Expression {
                    ty,
                    kind: ExpressionKind::Unary(op, Box::new(operand)),
                })
            }
            Expr::Binary(binary) if binary.attrs.is_empty() => {
                let op = match binary.op {
                    BinOp::Add(_) => BinaryOp::Add,
                    BinOp::Sub(_) => BinaryOp::Subtract,
                    BinOp::Mul(_) => BinaryOp::Multiply,
                    BinOp::Div(_) => BinaryOp::Divide,
                    BinOp::Rem(_) => BinaryOp::Remainder,
                    BinOp::Eq(_) => BinaryOp::Equal,
                    BinOp::Ne(_) => BinaryOp::NotEqual,
                    BinOp::Lt(_) => BinaryOp::Less,
                    BinOp::Le(_) => BinaryOp::LessEqual,
                    BinOp::Gt(_) => BinaryOp::Greater,
                    BinOp::Ge(_) => BinaryOp::GreaterEqual,
                    BinOp::And(_) => BinaryOp::And,
                    BinOp::Or(_) => BinaryOp::Or,
                    _ => return Err(TranspileError::Unsupported("不接受此二元運算")),
                };
                let left = self.expression(&binary.left)?;
                let right = self.expression(&binary.right)?;
                binary_expression(op, left, right)
            }
            _ => Err(TranspileError::Unsupported("不接受此值運算式或其屬性")),
        }
    }
}

/// 型別註記僅接受未限定路徑的 i32 與 bool。
fn parse_type(ty: &syn::Type) -> Result<Type, TranspileError> {
    if let syn::Type::Path(path) = ty
        && path.qself.is_none()
    {
        if path.path.is_ident("i32") {
            return Ok(Type::I32);
        }
        if path.path.is_ident("bool") {
            return Ok(Type::Bool);
        }
    }
    Err(TranspileError::Unsupported("型別僅接受 i32 與 bool"))
}

/// 比對型別，避免 C 的隱式整數／布林轉換掩蓋 Rust 錯誤。
fn same_type(expected: Type, actual: Type) -> Result<(), TranspileError> {
    if expected != actual {
        return Err(TranspileError::Semantic(format!(
            "型別不符：預期 {expected:?}，實際為 {actual:?}"
        )));
    }
    Ok(())
}

/// 依運算子檢查 operand 型別，並建立有確定結果型別的 IR。
fn binary_expression(
    op: BinaryOp,
    left: Expression,
    right: Expression,
) -> Result<Expression, TranspileError> {
    same_type(left.ty, right.ty)?;
    let ty = match op {
        BinaryOp::Equal | BinaryOp::NotEqual => Type::Bool,
        BinaryOp::And | BinaryOp::Or => {
            same_type(Type::Bool, left.ty)?;
            Type::Bool
        }
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            same_type(Type::I32, left.ty)?;
            Type::Bool
        }
        _ => {
            same_type(Type::I32, left.ty)?;
            Type::I32
        }
    };
    Ok(Expression {
        ty,
        kind: ExpressionKind::Binary(op, Box::new(left), Box::new(right)),
    })
}

/// 在不略過屬性驗證的前提下，尋找僅由括號包住的整數字面量。
fn literal_integer(expr: &Expr) -> Option<&syn::LitInt> {
    match expr {
        Expr::Lit(literal) if literal.attrs.is_empty() => match &literal.lit {
            Lit::Int(integer) => Some(integer),
            _ => None,
        },
        Expr::Paren(paren) if paren.attrs.is_empty() => literal_integer(&paren.expr),
        _ => None,
    }
}

/// 解析含進位／底線的整數，並在轉成 i32 前檢查範圍與後綴。
fn integer_value(integer: &syn::LitInt, negative: bool) -> Result<i32, TranspileError> {
    if !matches!(integer.suffix(), "" | "i32") {
        return Err(TranspileError::Unsupported("整數後綴僅接受 i32"));
    }
    let magnitude = integer
        .base10_parse::<i64>()
        .map_err(|_| TranspileError::Semantic("整數字面量超出 i32 範圍".into()))?;
    let value = if negative { -magnitude } else { magnitude };
    i32::try_from(value).map_err(|_| TranspileError::Semantic("整數字面量超出 i32 範圍".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// shadowing 的 initializer 應讀到舊 binding，離開內層後應恢復外層。
    #[test]
    fn assigns_distinct_ids_and_restores_scope() {
        let file = syn::parse_file("fn main() { let x = 1; { let x = x; } let y = x; }").unwrap();
        let program = crate::validate::lower(&file).unwrap();
        let Statement::Block(inner) = &program.statements[1] else {
            panic!("應為區塊");
        };
        assert!(matches!(
            &inner[0],
            Statement::Let {
                id: 1,
                value: Expression {
                    kind: ExpressionKind::Variable(0),
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            &program.statements[2],
            Statement::Let {
                id: 2,
                value: Expression {
                    kind: ExpressionKind::Variable(0),
                    ..
                },
                ..
            }
        ));
    }
}
