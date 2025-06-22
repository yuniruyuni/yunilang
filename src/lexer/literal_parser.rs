//! リテラルの解析

/// 文字列のエスケープシーケンスを処理
pub fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some('0') => result.push('\0'),
                Some('x') => {
                    // 16進数エスケープ（\xNN）
                    let mut hex = String::new();
                    if let Some(h1) = chars.next() {
                        hex.push(h1);
                    }
                    if let Some(h2) = chars.next() {
                        hex.push(h2);
                    }
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(byte as char);
                    } else {
                        // 無効な16進数エスケープ
                        result.push('\\');
                        result.push('x');
                        result.push_str(&hex);
                    }
                }
                Some('u') => {
                    // Unicodeエスケープ（\u{NNNN}）
                    if chars.next() == Some('{') {
                        let mut hex = String::new();
                        loop {
                            match chars.next() {
                                Some('}') => break,
                                Some(c) if c.is_ascii_hexdigit() => hex.push(c),
                                _ => {
                                    // 無効なUnicodeエスケープ
                                    result.push('\\');
                                    result.push('u');
                                    result.push('{');
                                    result.push_str(&hex);
                                    break;
                                }
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                            } else {
                                // 無効なUnicodeコードポイント
                                result.push_str(&format!("\\u{{{}}}", hex));
                            }
                        }
                    } else {
                        result.push('\\');
                        result.push('u');
                    }
                }
                Some(c) => {
                    // 認識されないエスケープシーケンス
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'), // 文字列の終端
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// 整数リテラルを解析（型サフィックス付き）
pub fn parse_integer_with_suffix(s: &str) -> Option<(i128, Option<String>)> {
    // 型サフィックスを見つける
    let suffixes = [
        "i8", "i16", "i32", "i64", "i128", "i256",
        "u8", "u16", "u32", "u64", "u128", "u256",
    ];
    
    for suffix in &suffixes {
        if s.ends_with(suffix) {
            let num_part = &s[..s.len() - suffix.len()];
            if let Ok(num) = num_part.parse::<i128>() {
                return Some((num, Some(suffix.to_string())));
            }
        }
    }
    
    // サフィックスなし
    s.parse::<i128>().ok().map(|n| (n, None))
}

/// 浮動小数点リテラルを解析（型サフィックス付き）
pub fn parse_float_with_suffix(s: &str) -> Option<(f64, Option<String>)> {
    // 型サフィックスを見つける
    let suffixes = ["f8", "f16", "f32", "f64"];
    
    for suffix in &suffixes {
        if s.ends_with(suffix) {
            let num_part = &s[..s.len() - suffix.len()];
            if let Ok(num) = num_part.parse::<f64>() {
                return Some((num, Some(suffix.to_string())));
            }
        }
    }
    
    // サフィックスなし
    s.parse::<f64>().ok().map(|n| (n, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_string() {
        assert_eq!(unescape_string("hello"), "hello");
        assert_eq!(unescape_string("hello\\nworld"), "hello\nworld");
        assert_eq!(unescape_string("\\t\\r\\n"), "\t\r\n");
        assert_eq!(unescape_string("\\\\"), "\\");
        assert_eq!(unescape_string("\\\""), "\"");
        assert_eq!(unescape_string("\\x41"), "A");
        assert_eq!(unescape_string("\\u{1F600}"), "😀");
    }

    #[test]
    fn test_parse_integer_with_suffix() {
        assert_eq!(parse_integer_with_suffix("42"), Some((42, None)));
        assert_eq!(parse_integer_with_suffix("42i32"), Some((42, Some("i32".to_string()))));
        assert_eq!(parse_integer_with_suffix("100u64"), Some((100, Some("u64".to_string()))));
        assert_eq!(parse_integer_with_suffix("-10i8"), Some((-10, Some("i8".to_string()))));
    }

    #[test]
    fn test_parse_float_with_suffix() {
        assert_eq!(parse_float_with_suffix("3.14"), Some((3.14, None)));
        assert_eq!(parse_float_with_suffix("3.14f32"), Some((3.14, Some("f32".to_string()))));
        assert_eq!(parse_float_with_suffix("2.718f64"), Some((2.718, Some("f64".to_string()))));
    }
}