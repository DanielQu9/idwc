use crate::ir::Program;

/// 從已驗證 IR 生成僅依賴 C 標準函式庫的 C17 原始碼。
pub(crate) fn generate(program: &Program) -> String {
    let literal = escape_string(&program.message);
    format!(
        "#include <stdio.h>\n\nint main(void) {{\n    puts(\"{literal}\");\n    return 0;\n}}\n"
    )
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
