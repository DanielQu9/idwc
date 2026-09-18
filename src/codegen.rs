use std::collections::BTreeSet;

use crate::ir::{
    BinaryOp, Expression, ExpressionKind, PrintPart, Program, Statement, Type, UnaryOp,
};

/// 從已驗證 IR 生成僅依賴 C 標準函式庫的 C17 原始碼。
pub(crate) fn generate(program: &Program) -> String {
    let mut generator = Generator {
        body: String::new(),
        indent: 1,
        next_temp: 0,
        typed: false,
        helpers: BTreeSet::new(),
    };
    generator.statements(&program.statements);
    generator.line("return 0;");
    let mut output = String::from("#include <stdio.h>\n");
    if generator.typed {
        output.push_str(
            "#include <stdbool.h>\n#include <stdint.h>\n#include <inttypes.h>\n#include <stdlib.h>\n",
        );
    }
    output.push('\n');
    output.push_str(&runtime_helpers(&generator.helpers));
    output.push_str("int main(void) {\n");
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
                    let value = self.expression(value);
                    self.line(&format!("idwc_v{id} = {value};"));
                }
                Statement::Block(statements) => {
                    self.line("{");
                    self.indent += 1;
                    self.statements(statements);
                    self.indent -= 1;
                    self.line("}");
                }
                Statement::Print { parts, arguments } => self.print(parts, arguments),
                Statement::Evaluate(expression) => {
                    let value = self.expression(expression);
                    self.line(&format!("(void)({value});"));
                }
            }
        }
    }

    /// 輸出前先依序求值所有參數，避免參數失敗時先寫出部分文字。
    fn print(&mut self, parts: &[PrintPart], arguments: &[Expression]) {
        if arguments.is_empty() {
            // 無參數格式只含文字，維持 v0.1.0 的可讀 puts 輸出。
            let mut text = String::new();
            for part in parts {
                if let PrintPart::Text(part) = part {
                    text.push_str(part);
                }
            }
            self.line(&format!("puts(\"{}\");", escape_string(&text)));
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
                PrintPart::Argument(index) => match arguments[*index].ty {
                    Type::I32 => self.line(&format!("printf(\"%\" PRId32, {});", values[*index])),
                    Type::Bool => self.line(&format!(
                        "fputs({} ? \"true\" : \"false\", stdout);",
                        values[*index]
                    )),
                },
                _ => {}
            }
        }
        self.line("putchar('\\n');");
    }

    /// 運算式產生原子值或已求值的暫存值；左右 operand 依序求值。
    fn expression(&mut self, expression: &Expression) -> String {
        self.typed = true;
        let value = match &expression.kind {
            ExpressionKind::Integer(value) => {
                return if *value == i32::MIN {
                    "INT32_MIN".into()
                } else if *value < 0 {
                    format!("(-INT32_C({}))", -value)
                } else {
                    format!("INT32_C({value})")
                };
            }
            ExpressionKind::Boolean(value) => return value.to_string(),
            ExpressionKind::Variable(id) => return format!("idwc_v{id}"),
            ExpressionKind::Unary(op, operand) => {
                let operand = self.expression(operand);
                match op {
                    UnaryOp::Not => format!("(!{operand})"),
                    UnaryOp::Negate => {
                        self.helpers.insert("neg");
                        format!("idwc_neg({operand})")
                    }
                }
            }
            ExpressionKind::Binary(op, left, right) => {
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
                match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {
                        let helper = match op {
                            BinaryOp::Add => "add",
                            BinaryOp::Subtract => "sub",
                            BinaryOp::Multiply => "mul",
                            BinaryOp::Divide => "div",
                            _ => "rem",
                        };
                        self.helpers.insert(helper);
                        format!("idwc_{helper}({left}, {right})")
                    }
                    _ => {
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
            }
        };
        let temp = self.temp_name();
        self.line(&format!(
            "const {} {temp} = {value};",
            c_type(expression.ty)
        ));
        temp
    }
}

/// 將 IR 的確定型別映射到 C 標準型別。
fn c_type(ty: Type) -> &'static str {
    match ty {
        Type::I32 => "int32_t",
        Type::Bool => "bool",
    }
}

/// 只生成實際使用的檢查函式；先提升至 int64_t，避免 C signed overflow。
fn runtime_helpers(helpers: &BTreeSet<&str>) -> String {
    if helpers.is_empty() {
        return String::new();
    }
    let mut output = String::from(
        "static void idwc_fail(const char *message) {\n    fflush(stdout);\n    fputs(message, stderr);\n    exit(101);\n}\n\n",
    );
    if helpers
        .iter()
        .any(|name| matches!(*name, "add" | "sub" | "mul" | "neg"))
    {
        output.push_str(
            "static int32_t idwc_checked(int64_t value) {\n    if (value < INT32_MIN || value > INT32_MAX) {\n        idwc_fail(\"idwc: integer overflow\\n\");\n    }\n    return (int32_t)value;\n}\n\n",
        );
    }
    for helper in helpers {
        match *helper {
            "neg" => output.push_str(
                "static int32_t idwc_neg(int32_t value) {\n    return idwc_checked(-(int64_t)value);\n}\n\n",
            ),
            "div" | "rem" => {
                let operator = if *helper == "div" { "/" } else { "%" };
                output.push_str(&format!(
                    "static int32_t idwc_{helper}(int32_t a, int32_t b) {{\n    if (b == 0) {{\n        idwc_fail(\"idwc: division by zero\\n\");\n    }}\n    if (a == INT32_MIN && b == -1) {{\n        idwc_fail(\"idwc: integer overflow\\n\");\n    }}\n    return a {operator} b;\n}}\n\n"
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
