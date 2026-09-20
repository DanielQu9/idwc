use std::collections::HashMap;

use syn::{BinOp, Expr, Lit, Pat, Stmt, UnOp, ext::IdentExt};

use crate::{
    TranspileError,
    ir::{
        ArrayElement, BinaryOp, Expression, ExpressionKind, Function, Parameter, Program,
        Statement, Type, UnaryOp,
    },
};

/// 名稱解析後的 binding；id 在離開 scope 後也不重用。
#[derive(Clone, Copy)]
struct Binding {
    id: usize,
    ty: Type,
    mutable: bool,
}

/// 以 scope 堆疊解析名稱，並在降低 AST 時檢查型別與可變性。
struct Analyzer<'a> {
    scopes: Vec<HashMap<String, Binding>>,
    next_id: usize,
    loop_depth: usize,
    functions: &'a HashMap<String, Callable>,
    return_type: Type,
}

/// 第一階段收集的函式簽名，允許向前呼叫與遞迴。
struct Callable {
    id: usize,
    parameters: Vec<Type>,
    return_type: Type,
}

/// 先註冊所有函式簽名，再以獨立 scope 分析各函式主體。
pub(crate) fn analyze(items: &[&syn::ItemFn]) -> Result<Program, TranspileError> {
    let mut functions = HashMap::new();
    for (id, function) in items.iter().enumerate() {
        let name = function.sig.ident.unraw().to_string();
        if !name.is_ascii() {
            return Err(TranspileError::Unsupported("函式識別字暫僅接受 ASCII"));
        }
        let mut parameters = Vec::new();
        for arg in &function.sig.inputs {
            let syn::FnArg::Typed(parameter) = arg else {
                return Err(TranspileError::Unsupported("不接受 self 參數"));
            };
            if !parameter.attrs.is_empty() {
                return Err(TranspileError::Unsupported("不接受參數屬性"));
            }
            binding_pattern(&parameter.pat)?;
            let ty = parse_type(&parameter.ty)?;
            require_parameter_type(ty)?;
            parameters.push(ty);
        }
        let return_type = match &function.sig.output {
            syn::ReturnType::Default => Type::Unit,
            syn::ReturnType::Type(_, ty) => parse_type(ty)?,
        };
        require_return_type(return_type)?;
        if functions
            .insert(
                name.clone(),
                Callable {
                    id,
                    parameters,
                    return_type,
                },
            )
            .is_some()
        {
            return Err(TranspileError::Semantic(format!("重複的函式 `{name}`")));
        }
    }
    if !functions.contains_key("main") {
        return Err(TranspileError::Semantic("缺少 fn main()".into()));
    }
    let mut program = Program {
        statements: Vec::new(),
        functions: Vec::new(),
    };
    for function in items {
        let name = function.sig.ident.unraw().to_string();
        let signature = &functions[&name];
        let mut analyzer = Analyzer {
            scopes: vec![HashMap::new()],
            next_id: 0,
            loop_depth: 0,
            functions: &functions,
            return_type: signature.return_type,
        };
        let mut parameters = Vec::new();
        for (arg, ty) in function.sig.inputs.iter().zip(&signature.parameters) {
            let syn::FnArg::Typed(parameter) = arg else {
                unreachable!("參數已驗證");
            };
            let ident = binding_pattern(&parameter.pat)?;
            let name = ident.ident.unraw().to_string();
            let binding = Binding {
                id: analyzer.next_id,
                ty: *ty,
                mutable: ident.mutability.is_some(),
            };
            analyzer.next_id += 1;
            if analyzer.scopes[0].insert(name.clone(), binding).is_some() {
                return Err(TranspileError::Semantic(format!("重複的參數 `{name}`")));
            }
            parameters.push(Parameter {
                id: binding.id,
                ty: binding.ty,
                mutable: binding.mutable,
            });
        }
        let statements = analyzer.function_body(&function.block)?;
        if signature.return_type != Type::Unit && !definitely_returns(&statements) {
            return Err(TranspileError::Semantic(format!(
                "函式 `{name}` 可能沒有回傳值"
            )));
        }
        if name == "main" {
            program.statements = statements;
        } else {
            program.functions.push(Function {
                id: signature.id,
                parameters,
                return_type: signature.return_type,
                statements,
            });
        }
    }
    Ok(program)
}

impl Analyzer<'_> {
    /// 有值函式的最後一個值運算式降低為 return，內部敘述區塊仍維持 unit。
    fn function_body(&mut self, block: &syn::Block) -> Result<Vec<Statement>, TranspileError> {
        self.scopes.push(HashMap::new());
        let result = (|| {
            let mut statements = Vec::new();
            for (index, statement) in block.stmts.iter().enumerate() {
                if index + 1 == block.stmts.len()
                    && let Stmt::Expr(expr, None) = statement
                {
                    let value = if self.return_type == Type::Unit {
                        self.expression(expr)
                    } else {
                        self.expression_expected(expr, self.return_type)
                    };
                    match value {
                        Ok(value) => {
                            if self.return_type == Type::Unit && value.ty != Type::Unit {
                                return Err(TranspileError::Unsupported(
                                    "unit 函式不能使用有值的尾運算式",
                                ));
                            }
                            same_type(self.return_type, value.ty)?;
                            if self.return_type == Type::Unit {
                                statements.push(Statement::Evaluate(value));
                            } else {
                                statements.push(Statement::Return(Some(value)));
                            }
                            continue;
                        }
                        Err(TranspileError::Unsupported(_)) => {}
                        Err(error) => return Err(error),
                    }
                }
                statements.push(self.statement(statement)?);
            }
            Ok(statements)
        })();
        self.scopes.pop();
        result
    }
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
            Stmt::Expr(Expr::Return(ret), _) if ret.attrs.is_empty() => {
                let value = ret
                    .expr
                    .as_ref()
                    .map(|expr| self.expression_expected(expr, self.return_type))
                    .transpose()?;
                same_type(
                    self.return_type,
                    value.as_ref().map_or(Type::Unit, |value| value.ty),
                )?;
                Ok(Statement::Return(value))
            }
            Stmt::Expr(Expr::Call(call), None) if call.attrs.is_empty() => {
                let value = self.call(call)?;
                same_type(Type::Unit, value.ty)?;
                Ok(Statement::Evaluate(value))
            }
            Stmt::Expr(Expr::Assign(assign), _) if assign.attrs.is_empty() => {
                if let Expr::Index(index) = assign.left.as_ref() {
                    let (binding, index, element) = self.array_index(index, true)?;
                    let value = self.expression_expected(&assign.right, element.ty())?;
                    return Ok(Statement::AssignIndex {
                        id: binding.id,
                        length: array_parts(binding.ty).expect("陣列 target 已驗證").1,
                        index,
                        value,
                        op: None,
                    });
                }
                let binding = self.assignment_target(&assign.left)?;
                let value = self.expression_expected(&assign.right, binding.ty)?;
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
                    if let Expr::Index(index) = binary.left.as_ref() {
                        let (binding, index, element) = self.array_index(index, true)?;
                        let right = self.expression_expected(&binary.right, element.ty())?;
                        validate_binary_types(op, element.ty(), right.ty)?;
                        return Ok(Statement::AssignIndex {
                            id: binding.id,
                            length: array_parts(binding.ty).expect("陣列 target 已驗證").1,
                            index,
                            value: right,
                            op: Some(op),
                        });
                    }
                    let binding = self.assignment_target(&binary.left)?;
                    let right = self.expression_expected(&binary.right, binding.ty)?;
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

    /// 條件只接受 bool 運算式，不使用 C 的整數 truthiness。
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

    /// 帶分號的運算式可捨棄結果，但仍保留副作用。
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
        let ident = binding_pattern(pattern)?;
        let init = local
            .init
            .as_ref()
            .ok_or(TranspileError::Unsupported("let 必須在宣告時初始化"))?;
        if init.diverge.is_some() {
            return Err(TranspileError::Unsupported("不接受 let-else"));
        }
        let value = if let Some(annotation) = annotation {
            self.expression_expected(&init.expr, annotation)?
        } else {
            self.expression(&init.expr)?
        };
        require_value(value.ty)?;
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

    /// print!／println! 接受基本值；固定精度格式另外要求 f64。
    fn print(&self, mac: &syn::Macro) -> Result<Statement, TranspileError> {
        let (parts, arguments) = crate::format::parse(mac)?;
        let arguments = arguments
            .iter()
            .map(|expr| self.expression(expr))
            .collect::<Result<Vec<_>, _>>()?;
        for argument in &arguments {
            require_scalar(argument.ty)?;
        }
        for part in &parts {
            if let crate::ir::PrintPart::Argument {
                index,
                precision: Some(_),
            } = part
            {
                same_type(Type::F64, arguments[*index].ty)?;
            }
        }
        Ok(Statement::Print {
            parts,
            arguments,
            newline: mac.path.is_ident("println"),
        })
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

    /// 遞迴檢查值運算式；所有未列出的 AST 與屬性皆拒絕。
    fn expression(&self, expr: &Expr) -> Result<Expression, TranspileError> {
        match expr {
            Expr::Tuple(tuple) if tuple.attrs.is_empty() && tuple.elems.is_empty() => {
                Ok(Expression {
                    ty: Type::Unit,
                    kind: ExpressionKind::Unit,
                })
            }
            Expr::Call(call) if call.attrs.is_empty() => self.call(call),
            Expr::Array(array) if array.attrs.is_empty() => self.array_literal(array, None),
            Expr::Repeat(repeat) if repeat.attrs.is_empty() => self.array_repeat(repeat, None),
            Expr::Index(index) if index.attrs.is_empty() => {
                let (binding, index, element) = self.array_index(index, false)?;
                let (_, length) = array_parts(binding.ty).expect("陣列 access 已驗證");
                Ok(Expression {
                    ty: element.ty(),
                    kind: ExpressionKind::Index {
                        id: binding.id,
                        length,
                        index: Box::new(index),
                    },
                })
            }
            Expr::MethodCall(call) if call.attrs.is_empty() => self.method_call(call),
            Expr::Lit(literal) if literal.attrs.is_empty() => match &literal.lit {
                Lit::Float(float) => {
                    if !matches!(float.suffix(), "" | "f64") {
                        return Err(TranspileError::Unsupported("浮點字面量僅接受 f64 後綴"));
                    }
                    float_expression(float.base10_digits())
                }
                Lit::Int(integer) if integer.suffix() == "f64" => {
                    let spelling = integer.to_string();
                    if spelling.starts_with("0x")
                        || spelling.starts_with("0o")
                        || spelling.starts_with("0b")
                    {
                        return Err(TranspileError::Unsupported("浮點字面量僅接受十進位"));
                    }
                    float_expression(integer.base10_digits())
                }
                Lit::Int(integer) => Ok(Expression {
                    ty: if integer.suffix() == "usize" {
                        Type::Usize
                    } else {
                        Type::I32
                    },
                    kind: if integer.suffix() == "usize" {
                        ExpressionKind::Usize(usize_value(integer)?)
                    } else {
                        ExpressionKind::Integer(integer_value(integer, false)?)
                    },
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
                    && integer.suffix() != "f64"
                {
                    return Ok(Expression {
                        ty,
                        kind: ExpressionKind::Integer(integer_value(integer, true)?),
                    });
                }
                let operand = self.expression(&unary.expr)?;
                if matches!(op, UnaryOp::Negate) {
                    require_signed_number(operand.ty)?;
                } else {
                    same_type(ty, operand.ty)?;
                }
                Ok(Expression {
                    ty: operand.ty,
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
                let (left, right) = if is_unsuffixed_integer(&binary.left) {
                    let right = self.expression(&binary.right)?;
                    (self.expression_expected(&binary.left, right.ty)?, right)
                } else {
                    let left = self.expression(&binary.left)?;
                    let right = self.expression_expected(&binary.right, left.ty)?;
                    (left, right)
                };
                binary_expression(op, left, right)
            }
            _ => Err(TranspileError::Unsupported("不接受此值運算式或其屬性")),
        }
    }

    /// 只在 Rust 會進行整數字面量推導的位置建立 usize；其餘整數仍預設 i32。
    fn expression_expected(
        &self,
        expr: &Expr,
        expected: Type,
    ) -> Result<Expression, TranspileError> {
        if expected == Type::Usize {
            if let Some(integer) = unsuffixed_integer(expr) {
                return usize_expression(integer);
            }
            if let Expr::Paren(paren) = expr
                && paren.attrs.is_empty()
            {
                return self.expression_expected(&paren.expr, expected);
            }
        }
        if let Expr::Array(array) = expr
            && array.attrs.is_empty()
        {
            return self.array_literal(array, Some(expected));
        }
        if let Expr::Repeat(repeat) = expr
            && repeat.attrs.is_empty()
        {
            return self.array_repeat(repeat, Some(expected));
        }
        let value = self.expression(expr)?;
        same_type(expected, value.ty)?;
        Ok(value)
    }

    fn array_literal(
        &self,
        array: &syn::ExprArray,
        expected: Option<Type>,
    ) -> Result<Expression, TranspileError> {
        if array.elems.len() > 4096 {
            return Err(TranspileError::Unsupported("固定陣列長度上限為 4096"));
        }
        if let Some(expected) = expected
            && array_parts(expected).is_none()
        {
            return Err(TranspileError::Semantic(format!(
                "型別不符：預期 {expected:?}，實際為陣列"
            )));
        }
        let expected_element =
            if let Some(element) = expected.and_then(array_parts).map(|parts| parts.0) {
                Some(element)
            } else {
                array
                    .elems
                    .iter()
                    .find(|expr| !is_unsuffixed_integer(expr))
                    .map(|expr| self.expression(expr))
                    .transpose()?
                    .map(|value| scalar_element(value.ty))
                    .transpose()?
            };
        if let Some((_, length)) = expected.and_then(array_parts)
            && length != array.elems.len()
        {
            return Err(TranspileError::Semantic("陣列長度與型別註記不符".into()));
        }
        let mut values = Vec::new();
        let mut element = expected_element;
        for expr in &array.elems {
            let value = if let Some(element) = element {
                self.expression_expected(expr, element.ty())?
            } else {
                self.expression(expr)?
            };
            let actual = scalar_element(value.ty)?;
            if let Some(element) = element {
                same_type(element.ty(), value.ty)?;
            } else {
                element = Some(actual);
            }
            values.push(value);
        }
        let element = element.ok_or(TranspileError::Unsupported("空陣列需要明確的型別註記"))?;
        Ok(Expression {
            ty: Type::Array(element, values.len()),
            kind: ExpressionKind::Array(values),
        })
    }

    fn array_repeat(
        &self,
        repeat: &syn::ExprRepeat,
        expected: Option<Type>,
    ) -> Result<Expression, TranspileError> {
        let length = array_length(&repeat.len)?;
        if let Some(expected) = expected
            && array_parts(expected).is_none()
        {
            return Err(TranspileError::Semantic(format!(
                "型別不符：預期 {expected:?}，實際為陣列"
            )));
        }
        let expected_parts = expected.and_then(array_parts);
        if let Some((_, expected_length)) = expected_parts
            && expected_length != length
        {
            return Err(TranspileError::Semantic("陣列長度與型別註記不符".into()));
        }
        let value = if let Some((element, _)) = expected_parts {
            self.expression_expected(&repeat.expr, element.ty())?
        } else {
            self.expression(&repeat.expr)?
        };
        let element = scalar_element(value.ty)?;
        Ok(Expression {
            ty: Type::Array(element, length),
            kind: ExpressionKind::ArrayRepeat(Box::new(value), length),
        })
    }

    fn array_index(
        &self,
        index: &syn::ExprIndex,
        mutable: bool,
    ) -> Result<(Binding, Expression, ArrayElement), TranspileError> {
        let binding = self.resolve(&index.expr)?;
        let (element, _) =
            array_parts(binding.ty).ok_or(TranspileError::Semantic("只能索引固定陣列".into()))?;
        if mutable && !binding.mutable {
            return Err(TranspileError::Semantic("不能修改不可變陣列".into()));
        }
        let index = self.expression_expected(&index.index, Type::Usize)?;
        Ok((binding, index, element))
    }

    fn method_call(&self, call: &syn::ExprMethodCall) -> Result<Expression, TranspileError> {
        if call.method != "len" || !call.args.is_empty() || call.turbofish.is_some() {
            return Err(TranspileError::Unsupported(
                "陣列方法目前僅支援無引數的 len()",
            ));
        }
        let binding = self.resolve(&call.receiver)?;
        let (_, length) = array_parts(binding.ty)
            .ok_or(TranspileError::Semantic("len() 目前僅支援固定陣列".into()))?;
        Ok(Expression {
            ty: Type::Usize,
            kind: ExpressionKind::ArrayLength(length),
        })
    }

    /// 檢查內建 I/O 與使用者函式呼叫的路徑、引數數量與型別。
    fn call(&self, call: &syn::ExprCall) -> Result<Expression, TranspileError> {
        let Expr::Path(path) = call.func.as_ref() else {
            return Err(TranspileError::Unsupported("不接受間接函式呼叫"));
        };
        if !path.attrs.is_empty()
            || path.qself.is_some()
            || path.path.leading_colon.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
        {
            return Err(TranspileError::Unsupported(
                "不接受呼叫屬性、限定型別或泛型引數",
            ));
        }
        let segments: Vec<_> = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if segments.len() == 3 && segments[0] == "idwc" && segments[1] == "io" {
            if !call.args.is_empty() {
                return Err(TranspileError::Semantic("內建 I/O 不接受引數".into()));
            }
            return match segments[2].as_str() {
                "read_i32" => Ok(Expression {
                    ty: Type::I32,
                    kind: ExpressionKind::ReadI32,
                }),
                "read_f64" => Ok(Expression {
                    ty: Type::F64,
                    kind: ExpressionKind::ReadF64,
                }),
                "flush_stdout" => Ok(Expression {
                    ty: Type::Unit,
                    kind: ExpressionKind::FlushStdout,
                }),
                _ => Err(TranspileError::Unsupported("不支援此內建 I/O 介面")),
            };
        }
        let ident = path.path.get_ident().ok_or(TranspileError::Unsupported(
            "函式呼叫僅接受單一識別字或指定的 I/O 路徑",
        ))?;
        let name = ident.unraw().to_string();
        if self
            .scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(&name))
        {
            return Err(TranspileError::Semantic(format!(
                "`{name}` 被變數遮蔽，不能呼叫"
            )));
        }
        if name == "main" {
            return Err(TranspileError::Unsupported("不接受呼叫 main"));
        }
        let function = self
            .functions
            .get(&name)
            .ok_or_else(|| TranspileError::Semantic(format!("找不到函式 `{name}`")))?;
        if function.parameters.len() != call.args.len() {
            return Err(TranspileError::Semantic(format!(
                "函式 `{name}` 引數數量不符"
            )));
        }
        let mut arguments = Vec::new();
        for (arg, ty) in call.args.iter().zip(&function.parameters) {
            let value = self.expression_expected(arg, *ty)?;
            same_type(*ty, value.ty)?;
            arguments.push(value);
        }
        Ok(Expression {
            ty: function.return_type,
            kind: ExpressionKind::Call(function.id, arguments),
        })
    }
}

/// 型別註記接受未限定路徑的基本型別，以及函式的 unit 回傳型別。
fn parse_type(ty: &syn::Type) -> Result<Type, TranspileError> {
    if let syn::Type::Tuple(tuple) = ty
        && tuple.elems.is_empty()
    {
        return Ok(Type::Unit);
    }
    if let syn::Type::Path(path) = ty
        && path.qself.is_none()
    {
        if path.path.is_ident("i32") {
            return Ok(Type::I32);
        }
        if path.path.is_ident("usize") {
            return Ok(Type::Usize);
        }
        if path.path.is_ident("bool") {
            return Ok(Type::Bool);
        }
        if path.path.is_ident("f64") {
            return Ok(Type::F64);
        }
    }
    if let syn::Type::Array(array) = ty {
        let element = scalar_element(parse_type(&array.elem)?)?;
        return Ok(Type::Array(element, array_length(&array.len)?));
    }
    Err(TranspileError::Unsupported(
        "型別僅接受 i32、usize、f64、bool、一維固定陣列與 unit 回傳型別",
    ))
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
    let ty = validate_binary_types(op, left.ty, right.ty)?;
    Ok(Expression {
        ty,
        kind: ExpressionKind::Binary(op, Box::new(left), Box::new(right)),
    })
}

fn validate_binary_types(op: BinaryOp, left: Type, right: Type) -> Result<Type, TranspileError> {
    same_type(left, right)?;
    require_scalar(left)?;
    let ty = match op {
        BinaryOp::Equal | BinaryOp::NotEqual => Type::Bool,
        BinaryOp::And | BinaryOp::Or => {
            same_type(Type::Bool, left)?;
            Type::Bool
        }
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            require_number(left)?;
            Type::Bool
        }
        BinaryOp::Remainder => {
            if left == Type::F64 {
                return Err(TranspileError::Unsupported("尚未支援浮點餘數"));
            }
            if !matches!(left, Type::I32 | Type::Usize) {
                return Err(TranspileError::Semantic(
                    "型別不符：餘數僅接受 i32 或 usize".into(),
                ));
            }
            left
        }
        _ => {
            require_number(left)?;
            left
        }
    };
    Ok(ty)
}

/// 浮點字面量採 binary64；無限大字面量拒絕翻譯，極小值可捨入至零。
fn float_expression(digits: &str) -> Result<Expression, TranspileError> {
    let value = digits
        .parse::<f64>()
        .map_err(|_| TranspileError::Semantic("無效的 f64 字面量".into()))?;
    if !value.is_finite() {
        return Err(TranspileError::Semantic(
            "浮點字面量超出 f64 有限範圍".into(),
        ));
    }
    Ok(Expression {
        ty: Type::F64,
        kind: ExpressionKind::Float(value),
    })
}

/// 算術接受相同型別的整數或浮點數，bool 不做隱式轉換。
fn require_number(ty: Type) -> Result<(), TranspileError> {
    if !matches!(ty, Type::I32 | Type::Usize | Type::F64) {
        return Err(TranspileError::Semantic(
            "型別不符：此運算僅接受 i32、usize 或 f64".into(),
        ));
    }
    Ok(())
}

fn require_signed_number(ty: Type) -> Result<(), TranspileError> {
    if !matches!(ty, Type::I32 | Type::F64) {
        return Err(TranspileError::Semantic(
            "型別不符：負號僅接受 i32 或 f64".into(),
        ));
    }
    Ok(())
}

fn require_scalar(ty: Type) -> Result<(), TranspileError> {
    if !matches!(ty, Type::I32 | Type::Usize | Type::F64 | Type::Bool) {
        return Err(TranspileError::Unsupported(
            "此處僅接受 i32、usize、f64 或 bool",
        ));
    }
    Ok(())
}

fn require_parameter_type(ty: Type) -> Result<(), TranspileError> {
    require_scalar(ty).map_err(|_| TranspileError::Unsupported("函式參數暫不支援陣列或 unit"))
}

fn require_return_type(ty: Type) -> Result<(), TranspileError> {
    if matches!(ty, Type::Array(_, _)) {
        return Err(TranspileError::Unsupported("函式回傳值暫不支援陣列"));
    }
    Ok(())
}

fn array_parts(ty: Type) -> Option<(ArrayElement, usize)> {
    match ty {
        Type::Array(element, length) => Some((element, length)),
        _ => None,
    }
}

fn scalar_element(ty: Type) -> Result<ArrayElement, TranspileError> {
    match ty {
        Type::I32 => Ok(ArrayElement::I32),
        Type::Usize => Ok(ArrayElement::Usize),
        Type::F64 => Ok(ArrayElement::F64),
        Type::Bool => Ok(ArrayElement::Bool),
        Type::Unit | Type::Array(_, _) => Err(TranspileError::Unsupported(
            "固定陣列元素僅接受 i32、usize、f64 或 bool",
        )),
    }
}

fn array_length(expr: &Expr) -> Result<usize, TranspileError> {
    let integer =
        literal_integer(expr).ok_or(TranspileError::Unsupported("固定陣列長度僅接受整數字面量"))?;
    let length = usize_value(integer)?;
    if length > 4096 {
        return Err(TranspileError::Unsupported("固定陣列長度上限為 4096"));
    }
    Ok(length)
}

/// unit 僅用於函式回傳，不建立 unit 變數、參數或格式引數。
fn require_value(ty: Type) -> Result<(), TranspileError> {
    if ty == Type::Unit {
        return Err(TranspileError::Unsupported("此處不接受 unit 值"));
    }
    Ok(())
}

/// binding 僅接受無屬性、無借用的 ASCII 識別字。
fn binding_pattern(pattern: &Pat) -> Result<&syn::PatIdent, TranspileError> {
    let Pat::Ident(ident) = pattern else {
        return Err(TranspileError::Unsupported("binding 僅接受單一識別字"));
    };
    if !ident.attrs.is_empty()
        || ident.by_ref.is_some()
        || ident.subpat.is_some()
        || !ident.ident.unraw().to_string().is_ascii()
    {
        return Err(TranspileError::Unsupported(
            "binding 僅接受無屬性、無借用的 ASCII 識別字",
        ));
    }
    Ok(ident)
}

/// 保守檢查有值函式的回傳路徑；迴圈本身不視為保證回傳。
fn definitely_returns(statements: &[Statement]) -> bool {
    for statement in statements {
        match statement {
            Statement::Return(_) => return true,
            Statement::Block(body) if definitely_returns(body) => return true,
            Statement::If {
                then_branch,
                else_branch: Some(else_branch),
                ..
            } if definitely_returns(then_branch) && definitely_returns(else_branch) => return true,
            Statement::Break | Statement::Continue => return false,
            _ => {}
        }
    }
    false
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

fn unsuffixed_integer(expr: &Expr) -> Option<&syn::LitInt> {
    literal_integer(expr).filter(|integer| integer.suffix().is_empty())
}

fn is_unsuffixed_integer(expr: &Expr) -> bool {
    unsuffixed_integer(expr).is_some()
}

fn usize_expression(integer: &syn::LitInt) -> Result<Expression, TranspileError> {
    Ok(Expression {
        ty: Type::Usize,
        kind: ExpressionKind::Usize(usize_value(integer)?),
    })
}

fn usize_value(integer: &syn::LitInt) -> Result<usize, TranspileError> {
    if !matches!(integer.suffix(), "" | "usize") {
        return Err(TranspileError::Unsupported(
            "usize 字面量僅接受無後綴或 usize 後綴",
        ));
    }
    integer
        .base10_parse::<usize>()
        .map_err(|_| TranspileError::Semantic("整數字面量超出 usize 範圍".into()))
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
