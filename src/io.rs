//! v0.4.0 的有限輸入／flush 介面，供 Rust 範例與生成的 C 共用規格。

use std::io::{Read, Write};

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
}

impl InputError {
    fn message(&self) -> &'static str {
        match self {
            Self::Eof => "idwc: unexpected EOF\n",
            Self::Io => "idwc: stdin I/O error\n",
            Self::Invalid => "idwc: invalid integer\n",
            Self::Range => "idwc: integer out of range\n",
            Self::TooLong => "idwc: input token too long\n",
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

fn read_from(reader: &mut impl Read) -> Result<i32, InputError> {
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

#[cfg(test)]
mod tests {
    use super::{InputError, read_from};
    use std::io::{Cursor, Read};

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
}
