//! 単相化（Monomorphization）のテスト

use yunilang::analyzer::monomorphize_program;
use yunilang::parser::Parser;
use yunilang::lexer::Lexer;

#[test]
fn test_monomorphization_passthrough() {
    // 現在の実装では、ジェネリクスを使用していないプログラムは
    // そのまま返される
    let input = r#"
package test

fn main() {
    let x: i32 = 42;
    println(x);
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // 単相化を実行
    let monomorphized = monomorphize_program(program.clone()).unwrap();
    
    // 現在の実装では、元のプログラムと同じものが返される
    assert_eq!(monomorphized.items.len(), program.items.len());
}

#[test]
fn test_generic_function_monomorphization() {
    // 将来的に実装される予定のテスト
    let input = r#"
package test

fn identity<T>(x: T): T {
    x
}

fn main() {
    let a = identity(42);      // identity_i32 が生成される
    let b = identity(3.14);    // identity_f64 が生成される
    let c = identity("hello"); // identity_string が生成される
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // 単相化を実行
    let monomorphized = monomorphize_program(program).unwrap();
    
    // 元の2つの関数（identity<T>とmain）から
    // 4つの関数（identity_i32, identity_f64, identity_string, main）が生成される
    assert_eq!(monomorphized.items.len(), 4);
}

#[test]
fn test_generic_struct_monomorphization() {
    // 将来的に実装される予定のテスト
    let input = r#"
package test

struct Vec<T> {
    data: [T],
    len: u64,
}

fn main() {
    let v1: Vec<i32> = Vec { data: [], len: 0 };    // Vec_i32 が生成される
    let v2: Vec<string> = Vec { data: [], len: 0 }; // Vec_string が生成される
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // 単相化を実行
    let monomorphized = monomorphize_program(program).unwrap();
    
    // 元の1つの構造体定義と1つの関数から
    // 2つの構造体（Vec_i32, Vec_string）と1つの関数が生成される
    assert_eq!(monomorphized.items.len(), 3);
}