//! エラーケースのパーサーテスト

use super::*;

#[test]
fn test_missing_package() {
    // package宣言が無い場合のエラー
    let source = r#"
    fn main() {
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_missing_semicolon() {
    // セミコロンが無い場合のエラー
    let source = r#"
    package main
    
    fn main() {
        let x = 42
        let y = 24;
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_missing_closing_brace() {
    // 閉じ括弧が無い場合のエラー
    let source = r#"
    package main
    
    fn main() {
        let x = 42;
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_missing_closing_paren() {
    // 閉じ括弧が無い場合のエラー
    let source = r#"
    package main
    
    fn add(a: i32, b: i32: i32 {
        return a + b;
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_invalid_expression() {
    // 不正な式のエラー
    let source = r#"
    package main
    
    fn main() {
        let x = + * 5;
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_invalid_function_syntax() {
    // 不正な関数構文のエラー
    let source = r#"
    package main
    
    fn {
        return 42;
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_invalid_type_syntax() {
    // 不正な型構文のエラー
    let source = r#"
    package main
    
    fn main() {
        let x: = 42;
    }
    "#;
    
    assert_parse_error(source);
}

#[test]
fn test_invalid_if_syntax() {
    // 不正なif構文のエラー
    let source = r#"
    package main
    
    fn main() {
        if {
            println("hello");
        }
    }
    "#;
    
    assert_parse_error(source);
}