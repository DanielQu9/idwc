use std::collections::BTreeSet;

use crate::ir::{
    ArrayElement, BinaryOp, Expression, ExpressionKind, Function, ParseSource, PrintPart, Program,
    Statement, Type, UnaryOp,
};

/// 從已驗證 IR 生成僅依賴 C 標準函式庫的 C17 原始碼。
pub(crate) fn generate(program: &Program) -> String {
    let mut generator = Generator {
        body: String::new(),
        indent: 1,
        next_temp: 0,
        typed: false,
        helpers: BTreeSet::new(),
        in_main: false,
        floating: false,
        target_usize: false,
    };
    let mut prototypes = String::new();
    let mut functions = String::new();
    for function in &program.functions {
        generator.typed = true;
        generator.floating |= function.return_type == Type::F64
            || function
                .parameters
                .iter()
                .any(|parameter| parameter.ty == Type::F64);
        generator.target_usize |= function.return_type == Type::Usize
            || function
                .parameters
                .iter()
                .any(|parameter| parameter.ty == Type::Usize);
        let signature = function_signature(function);
        prototypes.push_str(&format!("{signature};\n"));
        functions.push_str(&format!("{signature} {{\n"));
        for parameter in &function.parameters {
            generator.line(&format!("(void)idwc_v{};", parameter.id));
        }
        generator.statements(&function.statements);
        if function.return_type == Type::Unit {
            generator.line("return;");
        }
        functions.push_str(&generator.body);
        functions.push_str("}\n\n");
        generator.body.clear();
    }
    generator.in_main = true;
    generator.statements(&program.statements);
    generator.line("return 0;");
    if generator.floating {
        generator.helpers.insert("float");
    }
    let mut output = String::from("#include <stdio.h>\n");
    if generator.typed {
        output.push_str(
            "#include <stdbool.h>\n#include <stdint.h>\n#include <inttypes.h>\n#include <stdlib.h>\n",
        );
    }
    if !generator.helpers.is_empty() {
        output.push_str("#include <signal.h>\n");
    }
    if generator.floating {
        output.push_str("#include <float.h>\n#include <math.h>\n#include <fenv.h>\n#include <locale.h>\n#include <string.h>\n");
    }
    if generator.target_usize {
        output.push_str(&format!(
            "_Static_assert(SIZE_MAX == UINT{}_MAX, \"idwc usize requires matching Rust and C target widths\");\n",
            usize::BITS
        ));
    }
    output.push('\n');
    output.push_str(&runtime_helpers(&generator.helpers));
    if !prototypes.is_empty() {
        output.push_str(&prototypes);
        output.push('\n');
        output.push_str(&functions);
    }
    output.push_str("int main(void) {\n");
    if generator.floating {
        output.push_str("    idwc_float_init();\n");
    }
    output.push_str(&generator.body);
    output.push_str("}\n");
    output
}

/// 將運算式依 Rust 順序降低為 C 敘述，暫存值名稱在程式內唯一。
struct Generator {
    body: String,
    indent: usize,
    next_temp: usize,
    typed: bool,
    helpers: BTreeSet<&'static str>,
    in_main: bool,
    floating: bool,
    target_usize: bool,
}

impl Generator {
    /// 寫入目前縮排層級的一行 C。
    fn line(&mut self, line: &str) {
        self.body.push_str(&"    ".repeat(self.indent));
        self.body.push_str(line);
        self.body.push('\n');
    }

    /// 配置暫存值名稱，避免 C 引數不保證的求值順序。
    fn temp_name(&mut self) -> String {
        let name = format!("idwc_t{}", self.next_temp);
        self.next_temp += 1;
        name
    }

    /// 依序輸出敘述與區塊，保留 Rust 主體執行順序。
    fn statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            match statement {
                Statement::Let { id, mutable, value } => {
                    if matches!(value.ty, Type::Array(_, _)) {
                        self.array_let(*id, *mutable, value);
                        continue;
                    }
                    let initializer = self.expression(value);
                    let qualifier = if *mutable { "" } else { "const " };
                    self.line(&format!(
                        "{qualifier}{} idwc_v{id} = {initializer};",
                        c_type(value.ty)
                    ));
                    // Rust 允許未使用的 binding；避免 C 嚴格警告阻止編譯。
                    self.line(&format!("(void)idwc_v{id};"));
                }
                Statement::Assign { id, value } => {
                    if matches!(value.ty, Type::Array(_, _)) {
                        self.array_assign(*id, value);
                        continue;
                    }
                    let value = self.expression(value);
                    self.line(&format!("idwc_v{id} = {value};"));
                }
                Statement::AssignIndex {
                    id,
                    length,
                    index,
                    value,
                    op,
                } => {
                    // Rust assignment evaluates the right-hand side before the place expression.
                    let right = self.expression(value);
                    let index = self.checked_index(index, *length);
                    let result = if let Some(op) = op {
                        self.binary_value(*op, value.ty, &format!("idwc_v{id}[{index}]"), &right)
                    } else {
                        right
                    };
                    self.line(&format!("idwc_v{id}[{index}] = {result};"));
                }
                Statement::Block(statements) => {
                    self.line("{");
                    self.indent += 1;
                    self.statements(statements);
                    self.indent -= 1;
                    self.line("}");
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition = self.expression(condition);
                    self.line(&format!("if ({condition}) {{"));
                    self.indent += 1;
                    self.statements(then_branch);
                    self.indent -= 1;
                    if let Some(else_branch) = else_branch {
                        self.line("} else {");
                        self.indent += 1;
                        self.statements(else_branch);
                        self.indent -= 1;
                    }
                    self.line("}");
                }
                Statement::While { condition, body } => {
                    // 條件可能生成多個暫存敘述；放在迴圈內，continue 才會重新求值。
                    self.line("for (;;) {");
                    self.indent += 1;
                    let condition = self.expression(condition);
                    self.line(&format!("if (!({condition})) {{"));
                    self.indent += 1;
                    self.line("break;");
                    self.indent -= 1;
                    self.line("}");
                    self.statements(body);
                    self.indent -= 1;
                    self.line("}");
                }
                Statement::ForRange {
                    id,
                    ty,
                    mutable,
                    start,
                    end,
                    inclusive,
                    body,
                } => self.for_range(*id, *ty, *mutable, start, end, *inclusive, body),
                Statement::Loop(body) => {
                    self.line("for (;;) {");
                    self.indent += 1;
                    self.statements(body);
                    self.indent -= 1;
                    self.line("}");
                }
                Statement::Break => self.line("break;"),
                Statement::Continue => self.line("continue;"),
                Statement::Print {
                    parts,
                    arguments,
                    newline,
                } => self.print(parts, arguments, *newline),
                Statement::Return(expression) => {
                    if let Some(expression) = expression {
                        let value = self.expression(expression);
                        if expression.ty != Type::Unit {
                            self.line(&format!("return {value};"));
                            continue;
                        }
                    }
                    self.line(if self.in_main { "return 0;" } else { "return;" });
                }
                Statement::Evaluate(expression) => {
                    if matches!(expression.ty, Type::Array(_, _)) {
                        let _ = self.array_values(expression);
                        continue;
                    }
                    let value = self.expression(expression);
                    self.line(&format!("(void)({value});"));
                }
            }
        }
    }

    /// Range bounds 依 Rust 順序求值一次，iterator counter 與 body binding 分離。
    #[allow(clippy::too_many_arguments)]
    fn for_range(
        &mut self,
        id: usize,
        ty: Type,
        mutable: bool,
        start: &Expression,
        end: &Expression,
        inclusive: bool,
        body: &[Statement],
    ) {
        self.line("{");
        self.indent += 1;
        let start = self.snapshot(start);
        let end = self.snapshot(end);
        let iterator = self.temp_name();
        let update = match ty {
            Type::I32 => {
                self.helpers.insert("add");
                format!("idwc_add({iterator}, INT32_C(1))")
            }
            Type::Usize => {
                self.helpers.insert("uadd");
                format!("idwc_uadd({iterator}, (size_t)UINT64_C(1))")
            }
            _ => unreachable!("for range 型別已限制為 i32 或 usize"),
        };
        if inclusive {
            let active = self.temp_name();
            self.line(&format!("bool {active} = {start} <= {end};"));
            self.line(&format!(
                "for ({} {iterator} = {start}; {active}; {active} = {iterator} != {end}, {iterator} = {active} ? {update} : {iterator}) {{",
                c_type(ty)
            ));
        } else {
            self.line(&format!(
                "for ({} {iterator} = {start}; {iterator} < {end}; {iterator} = {update}) {{",
                c_type(ty)
            ));
        }
        self.indent += 1;
        let qualifier = if mutable { "" } else { "const " };
        self.line(&format!(
            "{qualifier}{} idwc_v{id} = {iterator};",
            c_type(ty)
        ));
        self.line(&format!("(void)idwc_v{id};"));
        self.statements(body);
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
    }

    /// 輸出前先依序求值所有參數，避免參數失敗時先寫出部分文字。
    fn print(&mut self, parts: &[PrintPart], arguments: &[Expression], newline: bool) {
        if arguments.is_empty() {
            // 無參數格式只含文字，維持 v0.1.0 的可讀 puts 輸出。
            let mut text = String::new();
            for part in parts {
                if let PrintPart::Text(part) = part {
                    text.push_str(part);
                }
            }
            if newline {
                self.line(&format!("puts(\"{}\");", escape_string(&text)));
            } else {
                self.line(&format!("fputs(\"{}\", stdout);", escape_string(&text)));
            }
            return;
        }
        let mut values = Vec::new();
        for argument in arguments {
            let value = self.expression(argument);
            let temp = self.temp_name();
            self.line(&format!("const {} {temp} = {value};", c_type(argument.ty)));
            values.push(temp);
        }
        for part in parts {
            match part {
                PrintPart::Text(text) if !text.is_empty() => {
                    self.line(&format!("fputs(\"{}\", stdout);", escape_string(text)));
                }
                PrintPart::Argument { index, precision } => match arguments[*index].ty {
                    Type::I32 => self.line(&format!("printf(\"%\" PRId32, {});", values[*index])),
                    Type::Usize => self.line(&format!("printf(\"%zu\", {});", values[*index])),
                    Type::Bool => self.line(&format!(
                        "fputs({} ? \"true\" : \"false\", stdout);",
                        values[*index]
                    )),
                    Type::F64 => {
                        self.helpers.insert("float_print");
                        self.line(&format!(
                            "idwc_print_f64({}, {});",
                            values[*index],
                            precision.map_or(-1, i32::from)
                        ));
                    }
                    Type::Unit | Type::Array(_, _) | Type::String | Type::Tokens => {
                        unreachable!("格式參數已驗證為 scalar")
                    }
                },
                _ => {}
            }
        }
        if newline {
            self.line("putchar('\\n');");
        }
    }

    /// 運算式產生原子值或已求值的暫存值；左右 operand 依序求值。
    fn expression(&mut self, expression: &Expression) -> String {
        self.typed = true;
        self.floating |= expression.ty == Type::F64;
        self.target_usize |= matches!(expression.ty, Type::Usize | Type::Array(_, _));
        let value = match &expression.kind {
            ExpressionKind::Unit => return "0".into(),
            ExpressionKind::ReadI32 => {
                self.helpers.insert("read");
                self.helpers.insert("token");
                "idwc_read_i32()".into()
            }
            ExpressionKind::ReadF64 => {
                self.helpers.insert("float_read");
                self.helpers.insert("token");
                "idwc_read_f64()".into()
            }
            ExpressionKind::Float(value) => {
                let bits = value.to_bits();
                let exponent = (bits >> 52) as i32;
                let mantissa = bits & ((1u64 << 52) - 1);
                return if exponent == 0 {
                    format!("0x{mantissa:x}p-1074")
                } else {
                    format!("0x{:x}p{}", mantissa | (1u64 << 52), exponent - 1075)
                };
            }
            ExpressionKind::FlushStdout => {
                self.helpers.insert("flush");
                self.line("idwc_flush_stdout();");
                return "0".into();
            }
            ExpressionKind::Call(id, arguments) => {
                let mut values = Vec::new();
                for argument in arguments {
                    let value = self.expression(argument);
                    let temp = self.temp_name();
                    self.line(&format!("const {} {temp} = {value};", c_type(argument.ty)));
                    values.push(temp);
                }
                let call = format!("idwc_f{id}({})", values.join(", "));
                if expression.ty == Type::Unit {
                    self.line(&format!("{call};"));
                    return "0".into();
                }
                call
            }
            ExpressionKind::Integer(value) => {
                return if *value == i32::MIN {
                    "INT32_MIN".into()
                } else if *value < 0 {
                    format!("(-INT32_C({}))", -value)
                } else {
                    format!("INT32_C({value})")
                };
            }
            ExpressionKind::Usize(value) => return format!("((size_t)UINT64_C({value}))"),
            ExpressionKind::Boolean(value) => return value.to_string(),
            ExpressionKind::Variable(id) => return format!("idwc_v{id}"),
            ExpressionKind::Index { id, length, index } => {
                let index = self.checked_index(index, *length);
                return format!("idwc_v{id}[{index}]");
            }
            ExpressionKind::ArrayLength(length) => {
                return format!("((size_t)UINT64_C({length}))");
            }
            ExpressionKind::StringNew => {
                self.helpers.insert("line");
                "((idwc_string){{0}, 0})".into()
            }
            ExpressionKind::ReadLine(id) => {
                self.helpers.insert("line");
                self.line(&format!("idwc_read_line(&idwc_v{id});"));
                return "0".into();
            }
            ExpressionKind::SplitWhitespace(id) => {
                self.helpers.insert("line");
                format!("idwc_split_whitespace(&idwc_v{id})")
            }
            ExpressionKind::TokensLength(id) => return format!("idwc_v{id}.length"),
            ExpressionKind::Parse { source } => {
                self.helpers.insert("line");
                let suffix = match expression.ty {
                    Type::I32 => {
                        self.helpers.insert("line_parse_i32");
                        "i32"
                    }
                    Type::F64 => {
                        self.helpers.insert("line_parse_f64");
                        "f64"
                    }
                    _ => unreachable!("parse 目標已限制為 i32 或 f64"),
                };
                match source {
                    ParseSource::TrimmedString(id) => {
                        format!("idwc_parse_trimmed_{suffix}(&idwc_v{id})")
                    }
                    ParseSource::Token { id, index } => {
                        let index = self.expression(index);
                        let temp = self.temp_name();
                        self.line(&format!("const size_t {temp} = {index};"));
                        format!("idwc_parse_token_{suffix}(&idwc_v{id}, {temp})")
                    }
                }
            }
            ExpressionKind::Powi2(value) => {
                let value = self.snapshot(value);
                return self.float_temp(&format!("({value} * {value})"));
            }
            ExpressionKind::Array(_) | ExpressionKind::ArrayRepeat(_, _) => {
                unreachable!("陣列值只由陣列敘述降低")
            }
            ExpressionKind::Unary(op, operand) => {
                let operand = self.expression(operand);
                match op {
                    UnaryOp::Not => format!("(!{operand})"),
                    UnaryOp::Negate => {
                        if expression.ty == Type::F64 {
                            format!("(-{operand})")
                        } else {
                            self.helpers.insert("neg");
                            format!("idwc_neg({operand})")
                        }
                    }
                }
            }
            ExpressionKind::Binary(op, left, right) => {
                let operand_type = left.ty;
                let left = self.expression(left);
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    // 右 operand 的算術檢查也必須放在短路分支內。
                    let temp = self.temp_name();
                    self.line(&format!("bool {temp} = {left};"));
                    let condition = if matches!(op, BinaryOp::And) {
                        temp.clone()
                    } else {
                        format!("!{temp}")
                    };
                    self.line(&format!("if ({condition}) {{"));
                    self.indent += 1;
                    let right = self.expression(right);
                    self.line(&format!("{temp} = {right};"));
                    self.indent -= 1;
                    self.line("}");
                    return temp;
                }
                let right = self.expression(right);
                self.binary_value(*op, operand_type, &left, &right)
            }
        };
        let temp = self.temp_name();
        let qualifier = if expression.ty == Type::F64 {
            "const volatile"
        } else {
            "const"
        };
        self.line(&format!(
            "{qualifier} {} {temp} = {value};",
            c_type(expression.ty)
        ));
        temp
    }

    /// volatile 儲存每一步 binary64 結果，避免跨敘述 FMA 或延伸精度融合。
    fn float_temp(&mut self, value: &str) -> String {
        let temp = self.temp_name();
        self.line(&format!("const volatile double {temp} = {value};"));
        temp
    }

    fn checked_index(&mut self, index: &Expression, length: usize) -> String {
        let value = self.expression(index);
        let temp = self.temp_name();
        self.line(&format!("const size_t {temp} = {value};"));
        self.helpers.insert("bounds");
        if length == 0 {
            self.line("idwc_fail(\"idwc: array index out of bounds\\n\");");
        } else {
            self.line(&format!(
                "if ({temp} >= ((size_t)UINT64_C({length}))) {{ idwc_fail(\"idwc: array index out of bounds\\n\"); }}"
            ));
        }
        temp
    }

    fn array_let(&mut self, id: usize, mutable: bool, value: &Expression) {
        let Type::Array(element, length) = value.ty else {
            unreachable!("array_let 僅接受陣列")
        };
        self.typed = true;
        self.floating |= element == ArrayElement::F64;
        self.target_usize = true;
        let values = self.array_values(value);
        let qualifier = if mutable { "" } else { "const " };
        let physical_length = length.max(1);
        let initializer = if values.is_empty() {
            "0".into()
        } else {
            values.join(", ")
        };
        self.line(&format!(
            "{qualifier}{} idwc_v{id}[{physical_length}] = {{{initializer}}};",
            c_element_type(element)
        ));
        self.line(&format!("(void)idwc_v{id};"));
    }

    fn array_assign(&mut self, id: usize, value: &Expression) {
        let Type::Array(element, length) = value.ty else {
            unreachable!("array_assign 僅接受陣列")
        };
        self.typed = true;
        self.floating |= element == ArrayElement::F64;
        self.target_usize = true;
        let values = self.array_values(value);
        for (index, value) in values.iter().take(length).enumerate() {
            self.line(&format!("idwc_v{id}[{index}] = {value};"));
        }
    }

    fn array_values(&mut self, value: &Expression) -> Vec<String> {
        let Type::Array(_, length) = value.ty else {
            unreachable!("array_values 僅接受陣列")
        };
        match &value.kind {
            ExpressionKind::Array(values) => {
                values.iter().map(|value| self.snapshot(value)).collect()
            }
            ExpressionKind::ArrayRepeat(value, repeat_length) => {
                debug_assert_eq!(length, *repeat_length);
                let value = self.snapshot(value);
                vec![value; length]
            }
            ExpressionKind::Variable(id) => (0..length)
                .map(|index| format!("idwc_v{id}[{index}]"))
                .collect(),
            _ => unreachable!("陣列值僅能來自字面量、重複初始化或陣列 binding"),
        }
    }

    fn snapshot(&mut self, expression: &Expression) -> String {
        let value = self.expression(expression);
        let temp = self.temp_name();
        let qualifier = if expression.ty == Type::F64 {
            "const volatile"
        } else {
            "const"
        };
        self.line(&format!(
            "{qualifier} {} {temp} = {value};",
            c_type(expression.ty)
        ));
        temp
    }

    fn binary_value(&mut self, op: BinaryOp, ty: Type, left: &str, right: &str) -> String {
        if ty == Type::F64
            && matches!(
                op,
                BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
            )
        {
            if matches!(op, BinaryOp::Divide) {
                self.helpers.insert("float_div");
                return self.float_temp(&format!("idwc_fdiv({left}, {right})"));
            }
            let operator = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                _ => unreachable!(),
            };
            return self.float_temp(&format!("({left} {operator} {right})"));
        }
        if matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder
        ) {
            let helper = match op {
                BinaryOp::Add => "add",
                BinaryOp::Subtract => "sub",
                BinaryOp::Multiply => "mul",
                BinaryOp::Divide => "div",
                BinaryOp::Remainder => "rem",
                _ => unreachable!(),
            };
            let helper = if ty == Type::Usize {
                match helper {
                    "add" => "uadd",
                    "sub" => "usub",
                    "mul" => "umul",
                    "div" => "udiv",
                    "rem" => "urem",
                    _ => unreachable!(),
                }
            } else {
                helper
            };
            self.helpers.insert(helper);
            return format!("idwc_{helper}({left}, {right})");
        }
        let operator = match op {
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            _ => unreachable!("短路運算已在前面處理"),
        };
        format!("({left} {operator} {right})")
    }
}

/// 將 IR 的確定型別映射到 C 標準型別。
fn c_type(ty: Type) -> &'static str {
    match ty {
        Type::I32 => "int32_t",
        Type::Usize => "size_t",
        Type::F64 => "double",
        Type::Bool => "bool",
        Type::Unit => "void",
        Type::Array(_, _) => unreachable!("C scalar 型別不接受陣列"),
        Type::String => "idwc_string",
        Type::Tokens => "idwc_tokens",
    }
}

fn c_element_type(element: ArrayElement) -> &'static str {
    c_type(element.ty())
}

fn function_signature(function: &Function) -> String {
    let parameters = if function.parameters.is_empty() {
        "void".into()
    } else {
        function
            .parameters
            .iter()
            .map(|parameter| {
                let qualifier = if parameter.mutable { "" } else { "const " };
                format!("{qualifier}{} idwc_v{}", c_type(parameter.ty), parameter.id)
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{} idwc_f{}({parameters})",
        c_type(function.return_type),
        function.id
    )
}

/// 只生成實際使用的檢查函式；先提升至 int64_t，避免 C signed overflow。
fn runtime_helpers(helpers: &BTreeSet<&str>) -> String {
    if helpers.is_empty() {
        return String::new();
    }
    let mut output = String::from(
        "static void idwc_fail(const char *message) {\n#ifdef SIGPIPE\n    signal(SIGPIPE, SIG_IGN);\n#endif\n    fflush(stdout);\n    fputs(message, stderr);\n    exit(101);\n}\n\n",
    );
    if helpers
        .iter()
        .any(|name| matches!(*name, "add" | "sub" | "mul" | "neg"))
    {
        output.push_str(
            "static int32_t idwc_checked(int64_t value) {\n    if (value < INT32_MIN || value > INT32_MAX) {\n        idwc_fail(\"idwc: integer overflow\\n\");\n    }\n    return (int32_t)value;\n}\n\n",
        );
    }
    if helpers.contains("token") {
        output.push_str(include_str!("runtime/token.c"));
    }
    if helpers.contains("line") {
        output.push_str(include_str!("runtime/line.c"));
    }
    for helper in helpers {
        match *helper {
            "token" | "bounds" | "line" => {}
            "line_parse_i32" => output.push_str(include_str!("runtime/line_parse_i32.c")),
            "line_parse_f64" => output.push_str(include_str!("runtime/line_parse_f64.c")),
            "float" => output.push_str(include_str!("runtime/float.c")),
            "float_print" => output.push_str(include_str!("runtime/float_print.c")),
            "float_read" => output.push_str(include_str!("runtime/float_read.c")),
            "float_div" => output.push_str("static double idwc_fdiv(double a, double b) {\n    if (b == 0.0) {\n        if (a == 0.0 || isnan(a)) { return NAN; }\n        return signbit(a) != signbit(b) ? -INFINITY : INFINITY;\n    }\n    return a / b;\n}\n\n"),
            "flush" => output.push_str(
                "static void idwc_flush_stdout(void) {\n#ifdef SIGPIPE\n    signal(SIGPIPE, SIG_IGN);\n#endif\n    if (fflush(stdout) == EOF || ferror(stdout)) {\n        idwc_fail(\"idwc: stdout flush error\\n\");\n    }\n}\n\n",
            ),
            "read" => output.push_str(INPUT_HELPER),
            "neg" => output.push_str(
                "static int32_t idwc_neg(int32_t value) {\n    return idwc_checked(-(int64_t)value);\n}\n\n",
            ),
            "div" | "rem" => {
                let operator = if *helper == "div" { "/" } else { "%" };
                output.push_str(&format!(
                    "static int32_t idwc_{helper}(int32_t a, int32_t b) {{\n    if (b == 0) {{\n        idwc_fail(\"idwc: division by zero\\n\");\n    }}\n    if (a == INT32_MIN && b == -1) {{\n        idwc_fail(\"idwc: integer overflow\\n\");\n    }}\n    return a {operator} b;\n}}\n\n"
                ));
            }
            "uadd" => output.push_str(
                "static size_t idwc_uadd(size_t a, size_t b) {\n    if (a > SIZE_MAX - b) { idwc_fail(\"idwc: integer overflow\\n\"); }\n    return a + b;\n}\n\n",
            ),
            "usub" => output.push_str(
                "static size_t idwc_usub(size_t a, size_t b) {\n    if (a < b) { idwc_fail(\"idwc: integer overflow\\n\"); }\n    return a - b;\n}\n\n",
            ),
            "umul" => output.push_str(
                "static size_t idwc_umul(size_t a, size_t b) {\n    if (b != 0 && a > SIZE_MAX / b) { idwc_fail(\"idwc: integer overflow\\n\"); }\n    return a * b;\n}\n\n",
            ),
            "udiv" | "urem" => {
                let operator = if *helper == "udiv" { "/" } else { "%" };
                output.push_str(&format!(
                    "static size_t idwc_{helper}(size_t a, size_t b) {{\n    if (b == 0) {{ idwc_fail(\"idwc: division by zero\\n\"); }}\n    return a {operator} b;\n}}\n\n"
                ));
            }
            _ => {
                let operator = match *helper {
                    "add" => "+",
                    "sub" => "-",
                    "mul" => "*",
                    _ => unreachable!("helper 名稱由產生器配置"),
                };
                output.push_str(&format!(
                    "static int32_t idwc_{helper}(int32_t a, int32_t b) {{\n    return idwc_checked((int64_t)a {operator} (int64_t)b);\n}}\n\n"
                ));
            }
        }
    }
    output
}

const INPUT_HELPER: &str = r#"static int32_t idwc_read_i32(void) {
    unsigned char token[129];
    size_t length = idwc_read_token(token);
    bool negative = token[0] == '-';
    size_t start = (negative || token[0] == '+') ? 1 : 0;
    if (start == length) {
        idwc_fail("idwc: invalid integer\n");
    }
    for (size_t i = start; i < length; ++i) {
        if (token[i] < '0' || token[i] > '9') {
            idwc_fail("idwc: invalid integer\n");
        }
    }
    uint64_t limit = negative ? UINT64_C(2147483648) : UINT64_C(2147483647);
    uint64_t value = 0;
    for (size_t i = start; i < length; ++i) {
        uint64_t digit = token[i] - '0';
        if (value > (limit - digit) / 10) {
            idwc_fail("idwc: integer out of range\n");
        }
        value = value * 10 + digit;
    }
    if (negative && value == UINT64_C(2147483648)) {
        return INT32_MIN;
    }
    return negative ? -(int32_t)value : (int32_t)value;
}

"#;

/// 以 UTF-8 位元組編碼 C 字串；固定三位八進位避免吞掉後續數字。
fn escape_string(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        match byte {
            b'"' => result.push_str("\\\""),
            b'\\' => result.push_str("\\\\"),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            // C17 的 trigraph 在字串內也會處理，跳脫問號避免改寫。
            b'?' => result.push_str("\\?"),
            0x20..=0x7e => result.push(char::from(byte)),
            _ => {
                result.push('\\');
                result.push(char::from(b'0' + (byte >> 6)));
                result.push(char::from(b'0' + ((byte >> 3) & 7)));
                result.push(char::from(b'0' + (byte & 7)));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::escape_string;

    /// 非 ASCII 位元組及後接數字的控制字元不能形成含糊的 C 跳脫。
    #[test]
    fn escapes_bytes_without_consuming_following_digits() {
        assert_eq!(escape_string("\u{1}7é"), "\\0017\\303\\251");
        assert_eq!(escape_string("\"\\\n\r\t??/"), "\\\"\\\\\\n\\r\\t\\?\\?/");
    }
}
