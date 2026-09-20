//! 有限的數值／文字輸入與 flush 介面，供 Rust 範例與生成的 C 共用規格。

use std::io::{BufRead, Read, Write};

const LINE_LIMIT: usize = 4096;

/// 讀取下一個由 ASCII 空白分隔的十進位 `i32`。
///
/// 允許前置 `+`／`-`，token 最多 128 bytes；不接受數字後綴或部分解析。
/// ASCII 空白包括空格、tab、LF、CR、vertical tab 與 form feed。
/// EOF、I/O、格式、範圍或長度錯誤會 flush stdout、輸出診斷並以狀態 101 結束。
pub fn read_i32() -> i32 {
    match read_from(&mut std::io::stdin().lock()) {
        Ok(value) => value,
        Err(error) => fail(error.message()),
    }
}

/// 讀取下一個 ASCII 空白分隔的 `f64` token，最多 128 bytes。
///
/// 接受十進位／科學記號及 `NaN`、`inf`、`+inf`、`-inf`。
/// 數字溢位至無限大或非零數字下溢至零，皆回報範圍錯誤；非零 subnormal 可接受。
/// EOF、I/O、格式、範圍或長度錯誤會輸出診斷並以狀態 101 結束。
pub fn read_f64() -> f64 {
    match read_float_from(&mut std::io::stdin().lock()) {
        Ok(value) => value,
        Err(error) => fail(error.message()),
    }
}

/// 讀取一整行 UTF-8 文字並保留結尾換行。
///
/// 空 EOF 回傳空 [`String`]。輸入最多 4096 bytes；I/O、UTF-8 或長度錯誤
/// 會 flush stdout、輸出診斷並以狀態 101 結束。
pub fn read_line() -> String {
    match read_line_from(&mut std::io::stdin().lock()) {
        Ok(value) => value,
        Err(error) => fail(error.message()),
    }
}

/// Flush stdout，讓沒有換行的提示文字立即送出。
///
/// Flush 失敗會輸出診斷並以狀態 101 結束。
pub fn flush_stdout() {
    if std::io::stdout().lock().flush().is_err() {
        fail("idwc: stdout flush error\n");
    }
}

fn fail(message: &str) -> ! {
    let _ = std::io::stdout().lock().flush();
    let _ = std::io::stderr().lock().write_all(message.as_bytes());
    std::process::exit(101)
}

#[derive(Debug, PartialEq)]
enum InputError {
    Eof,
    Io,
    Invalid,
    Range,
    TooLong,
    InvalidFloat,
    FloatRange,
    InvalidUtf8,
    LineTooLong,
}

impl InputError {
    fn message(&self) -> &'static str {
        match self {
            Self::Eof => "idwc: unexpected EOF\n",
            Self::Io => "idwc: stdin I/O error\n",
            Self::Invalid => "idwc: invalid integer\n",
            Self::Range => "idwc: integer out of range\n",
            Self::TooLong => "idwc: input token too long\n",
            Self::InvalidFloat => "idwc: invalid float\n",
            Self::FloatRange => "idwc: float out of range\n",
            Self::InvalidUtf8 => "idwc: invalid UTF-8 input\n",
            Self::LineTooLong => "idwc: input line buffer too long\n",
        }
    }
}

fn read_byte(reader: &mut impl Read) -> Result<Option<u8>, InputError> {
    let mut byte = [0];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return Ok(None),
            Ok(_) => return Ok(Some(byte[0])),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(InputError::Io),
        }
    }
}

fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// 以固定大小緩衝區完整讀取 token，整數與浮點共用長度／I/O 規則。
fn read_token(reader: &mut impl Read) -> Result<([u8; 128], usize), InputError> {
    let mut byte = loop {
        let byte = read_byte(reader)?.ok_or(InputError::Eof)?;
        if !space(byte) {
            break byte;
        }
    };
    let mut token = [0; 128];
    let mut length = 0;
    loop {
        if length == token.len() {
            return Err(InputError::TooLong);
        }
        token[length] = byte;
        length += 1;
        match read_byte(reader)? {
            Some(next) if !space(next) => byte = next,
            _ => break,
        }
    }
    Ok((token, length))
}

/// 完整驗證十進位整數後才進行有界累積。
fn read_from(reader: &mut impl Read) -> Result<i32, InputError> {
    let (token, length) = read_token(reader)?;
    let negative = token[0] == b'-';
    let start = usize::from(negative || token[0] == b'+');
    let digits = &token[start..length];
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(InputError::Invalid);
    }
    let limit = if negative {
        2_147_483_648u64
    } else {
        2_147_483_647
    };
    let mut value = 0u64;
    for byte in digits {
        let digit = u64::from(byte - b'0');
        if value > (limit - digit) / 10 {
            return Err(InputError::Range);
        }
        value = value * 10 + digit;
    }
    if negative && value == 2_147_483_648 {
        return Ok(i32::MIN);
    }
    let value = i32::try_from(value).map_err(|_| InputError::Range)?;
    Ok(if negative { -value } else { value })
}

/// 以白名單驗證浮點 token，再用 Rust 的 binary64 解析器。
fn read_float_from(reader: &mut impl Read) -> Result<f64, InputError> {
    let (token, length) = read_token(reader)?;
    let token = &token[..length];
    match token {
        b"NaN" => return Ok(f64::NAN),
        b"inf" | b"+inf" => return Ok(f64::INFINITY),
        b"-inf" => return Ok(f64::NEG_INFINITY),
        _ => {}
    }
    let mut index = usize::from(matches!(token[0], b'+' | b'-'));
    let mut digits = 0;
    let mut nonzero = false;
    while index < length && token[index].is_ascii_digit() {
        nonzero |= token[index] != b'0';
        digits += 1;
        index += 1;
    }
    if token.get(index) == Some(&b'.') {
        index += 1;
        while index < length && token[index].is_ascii_digit() {
            nonzero |= token[index] != b'0';
            digits += 1;
            index += 1;
        }
    }
    if digits == 0 {
        return Err(InputError::InvalidFloat);
    }
    if matches!(token.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(token.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while index < length && token[index].is_ascii_digit() {
            index += 1;
        }
        if start == index {
            return Err(InputError::InvalidFloat);
        }
    }
    if index != length {
        return Err(InputError::InvalidFloat);
    }
    let text = std::str::from_utf8(token).map_err(|_| InputError::InvalidFloat)?;
    let value = text.parse::<f64>().map_err(|_| InputError::InvalidFloat)?;
    if !value.is_finite() || (nonzero && value == 0.0) {
        return Err(InputError::FloatRange);
    }
    Ok(value)
}

fn read_line_from(reader: &mut impl BufRead) -> Result<String, InputError> {
    let mut bytes = Vec::with_capacity(LINE_LIMIT + 1);
    (&mut *reader)
        .take((LINE_LIMIT + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|_| InputError::Io)?;
    if bytes.len() > LINE_LIMIT {
        return Err(InputError::LineTooLong);
    }
    String::from_utf8(bytes).map_err(|_| InputError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::{InputError, read_float_from, read_from, read_line_from};
    use std::io::{BufReader, Cursor, Read};

    #[test]
    fn repeated_reads_and_boundaries() {
        let mut input = Cursor::new(b"\x0b\x0c +2147483647\r\n-2147483648\t-0 007");
        for expected in [i32::MAX, i32::MIN, 0, 7] {
            assert_eq!(read_from(&mut input), Ok(expected));
        }
        assert_eq!(read_from(&mut input), Err(InputError::Eof));
        assert_eq!(read_from(&mut Cursor::new(vec![b'0'; 128])), Ok(0));
        assert_eq!(
            read_from(&mut Cursor::new(vec![b'0'; 129])),
            Err(InputError::TooLong)
        );
    }

    #[test]
    fn rejects_partial_numbers_and_overflow() {
        for input in ["+", "-", "12x", "1.0", "0xff", "1_000", "１２", "1\0"] {
            assert_eq!(read_from(&mut input.as_bytes()), Err(InputError::Invalid));
        }
        for input in ["2147483648", "-2147483649", "999999999999999999999"] {
            assert_eq!(read_from(&mut input.as_bytes()), Err(InputError::Range));
        }
    }

    #[test]
    fn distinguishes_io_errors_and_retries_interrupted_reads() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("test failure"))
            }
        }
        assert_eq!(read_from(&mut Broken), Err(InputError::Io));
        struct InterruptedOnce(bool, Cursor<&'static [u8]>);
        impl Read for InterruptedOnce {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if !self.0 {
                    self.0 = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                self.1.read(buffer)
            }
        }
        assert_eq!(
            read_from(&mut InterruptedOnce(false, Cursor::new(b"42"))),
            Ok(42)
        );
    }

    #[test]
    fn validates_float_tokens_and_range() {
        for (input, expected) in [
            (".5", 0.5),
            ("+1.", 1.0),
            ("-1e2", -100.0),
            ("5e-324", f64::from_bits(1)),
        ] {
            assert_eq!(read_float_from(&mut input.as_bytes()), Ok(expected));
        }
        for input in [
            ".", "+", "1e", "1e+", "1.2x", "0x1p2", "nan", "Infinity", "1_0", "１", "1\0",
        ] {
            assert_eq!(
                read_float_from(&mut input.as_bytes()),
                Err(InputError::InvalidFloat)
            );
        }
        for input in ["1e309", "-1e309", "1e-9999", "2e-324"] {
            assert_eq!(
                read_float_from(&mut input.as_bytes()),
                Err(InputError::FloatRange)
            );
        }
        assert!(read_float_from(&mut "NaN".as_bytes()).unwrap().is_nan());
        assert!(
            read_float_from(&mut "-0e9999".as_bytes())
                .unwrap()
                .is_sign_negative()
        );
    }

    #[test]
    fn reads_bounded_utf8_lines() {
        let mut input = Cursor::new("哈囉\nsecond".as_bytes());
        assert_eq!(read_line_from(&mut input), Ok("哈囉\n".into()));
        assert_eq!(read_line_from(&mut input), Ok("second".into()));
        assert_eq!(read_line_from(&mut input), Ok(String::new()));

        assert_eq!(
            read_line_from(&mut Cursor::new(vec![b'a'; 4096])),
            Ok("a".repeat(4096))
        );
        assert_eq!(
            read_line_from(&mut Cursor::new(vec![b'a'; 4097])),
            Err(InputError::LineTooLong)
        );
        assert_eq!(
            read_line_from(&mut Cursor::new(b"\xff\n")),
            Err(InputError::InvalidUtf8)
        );

        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("test failure"))
            }
        }
        assert_eq!(
            read_line_from(&mut BufReader::new(Broken)),
            Err(InputError::Io)
        );
    }
}
