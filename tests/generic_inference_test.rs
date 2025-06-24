//! ジェネリック型推論のテスト

use yunilang::parser::Parser;
use yunilang::lexer::Lexer;
use yunilang::analyzer::SemanticAnalyzer;

#[test]
fn test_generic_function_call_inference() {
    let input = r#"
package test

fn identity<T>(x: T): T {
    return x;
}

fn main() {
    let a = identity(42);       // T = i32
    let b = identity("hello");  // T = String
    let c = identity(true);     // T = bool
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_ok(), "Analysis failed: {:?}", result);
}

#[test]
fn test_generic_struct_literal_inference() {
    let input = r#"
package test

struct Pair<T, U> {
    first: T,
    second: U,
}

fn main() {
    let p1 = Pair { first: 10, second: "hello" };     // T = i32, U = String
    let p2 = Pair { first: true, second: 3.14 };      // T = bool, U = f64
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_ok(), "Analysis failed: {:?}", result);
}

#[test]
fn test_nested_generic_types() {
    let input = r#"
package test

struct Vec<T> {
    data: [T],
    len: u64,
}

struct Option<T> {
    value: T,
    has_value: bool,
}

fn wrap<T>(value: T): Option<T> {
    return Option { value: value, has_value: true };
}

fn main() {
    let vec_opt = wrap(Vec { data: [1, 2, 3], len: 3u64 });  // Option<Vec<i32>>
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_ok(), "Analysis failed: {:?}", result);
}

#[test]
fn test_generic_field_access() {
    let input = r#"
package test

struct Box<T> {
    value: T,
}

fn main() {
    let int_box = Box { value: 42 };
    let x: i32 = int_box.value;  // フィールドアクセスで正しい型を取得
    
    let str_box = Box { value: "hello" };
    let s: String = str_box.value;  // フィールドアクセスで正しい型を取得
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_ok(), "Analysis failed: {:?}", result);
}

#[test]
fn test_type_inference_error() {
    let input = r#"
package test

fn identity<T>(x: T): T {
    return x;
}

fn main() {
    let x: i32 = identity("hello");  // 型不一致エラー
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_err(), "Expected type error but analysis succeeded");
}