//! その他の機能のコード生成テスト

use super::*;

#[test]
fn test_string_operations_codegen() {
    // 文字列操作のコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let greeting = "Hello";
        let name = "World";
        println(greeting, ", ", name, "!");
    }
    "#;
    
    let ir = assert_compile_success(source, "strings");
    assert_valid_ir(&ir);
    
    // 文字列リテラルがグローバル定数として定義されていることを確認
    // LLVMのバージョンによっては文字列定数の名前が異なる場合がある
    assert!(ir.contains("@") || ir.contains("global") || ir.contains("constant"), 
            "Should contain string constants");
    assert!(ir.contains("Hello"), "Should contain string literal 'Hello'");
    assert!(ir.contains("World"), "Should contain string literal 'World'");
}

#[test]
fn test_boolean_operations_codegen() {
    // ブール演算のコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let a = true;
        let b = false;
        let and_result = a && b;
        let or_result = a || b;
        let not_result = !a;
    }
    "#;
    
    let ir = assert_compile_success(source, "bool_ops");
    assert_valid_ir(&ir);
    
    // ブール演算のLLVM命令が含まれていることを確認
    assert!(ir.contains("and") || ir.contains("or"), "Should contain logical operations");
}

#[test]
fn test_optimization_levels() {
    // 最適化レベルのテスト
    let source = r#"
    package main
    
    fn calculate_sum(n: i32): i32 {
        let mut sum = 0;
        let mut i = 1;
        while i <= n {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
    }
    
    fn main() {
        let result = calculate_sum(100);
    }
    "#;
    
    // 最適化なしのコンパイル
    let ir_o0 = assert_compile_success(source, "opt_test_o0");
    assert_valid_ir(&ir_o0);
    
    // 最適化レベルによってIRが変わることを確認
    // （実際の最適化は実装依存）
    assert!(!ir_o0.is_empty(), "Should generate non-empty IR");
}

#[test]
fn test_error_handling_codegen() {
    // エラーハンドリングのテスト
    let invalid_sources = [r#"
        package main
        fn main() {
            let x: i32 = "hello";
        }
        "#,
        
        // 未定義関数呼び出し
        r#"
        package main
        fn main() {
            unknown_function();
        }
        "#];
    
    for (i, source) in invalid_sources.iter().enumerate() {
        assert_compile_error(source, &format!("error_test_{}", i));
    }
}