//! 基本的なプログラム構造のコード生成テスト

use super::*;

#[test]
fn test_minimal_program_codegen() {
    // 最小限のプログラムのコード生成テスト
    let source = r#"
    package main
    
    fn main() {
    }
    "#;
    
    let ir = assert_compile_success(source, "minimal");
    assert_valid_ir(&ir);
    
    // main関数が存在することを確認
    assert!(ir.contains("define"), "Should contain main function definition");
    assert!(ir.contains("ret"), "Should contain return statement");
}

#[test]
fn test_hello_world_codegen() {
    // Hello Worldプログラムのコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        println("Hello, World!");
    }
    "#;
    
    let ir = assert_compile_success(source, "hello");
    assert_valid_ir(&ir);
    
    // printf呼び出しが含まれていることを確認
    assert!(ir.contains("printf"), "Should contain printf call for println");
    assert!(ir.contains("Hello, World!"), "Should contain the string literal");
}

#[test]
fn test_ir_validation() {
    // 生成されるLLVM IRの妥当性を確認
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    
    fn main() {
        let x = add(5, 3);
        println("Result:", x);
    }
    "#;
    
    let ir = assert_compile_success(source, "validation");
    
    // IRの基本構造チェック
    // LLVMのバージョンやプラットフォームによっては、targetのフォーマットが異なる場合がある
    assert!(ir.contains("target") || ir.contains("module asm"), "Should contain target information or module header");
    
    // 関数定義の存在確認
    assert!(ir.contains("define"), "Should contain function definitions");
    assert!(ir.matches("define").count() >= 2, "Should have at least 2 functions");
    
    // 基本ブロックとターミネータの確認
    assert!(ir.contains("entry:"), "Should contain entry blocks");
    assert!(ir.contains("ret"), "Should contain return instructions");
    
    // 文字列定数の確認
    assert!(ir.contains("@") || ir.contains("global"), "Should contain global definitions");
    assert!(ir.contains("Result:"), "Should contain the format string");
}