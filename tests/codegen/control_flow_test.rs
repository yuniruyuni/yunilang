//! 制御フローのコード生成テスト

use super::*;

#[test]
fn test_conditional_statements_codegen() {
    // 条件文のコード生成テスト
    let source = r#"
    package main
    
    fn abs(x: i32): i32 {
        if x < 0 {
            return -x;
        } else {
            return x;
        }
    }
    
    fn main() {
        let result = abs(-42);
    }
    "#;
    
    let ir = assert_compile_success(source, "conditionals");
    assert_valid_ir(&ir);
    
    // 条件分岐のLLVM命令が含まれていることを確認
    assert!(ir.contains("icmp"), "Should contain integer comparison");
    assert!(ir.contains("br"), "Should contain branch instructions");
    assert!(ir.contains("label"), "Should contain basic block labels");
}

#[test]
fn test_loops_codegen() {
    // ループのコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let mut i = 0;
        while i < 10 {
            println(i);
            i = i + 1;
        }
    }
    "#;
    
    let ir = assert_compile_success(source, "loops");
    assert_valid_ir(&ir);
    
    // ループのLLVM構造が含まれていることを確認
    assert!(ir.contains("br"), "Should contain branch instructions for loop");
    assert!(ir.contains("icmp"), "Should contain comparison for loop condition");
    assert!(ir.matches("label").count() >= 2, "Should contain multiple basic blocks for loop");
}

#[test]
fn test_multiple_return_paths_codegen() {
    // 複数の戻り値パスのコード生成テスト
    let source = r#"
    package main
    
    fn classify(x: i32): i32 {
        if x > 0 {
            return 1;
        } else if x < 0 {
            return -1;
        } else {
            return 0;
        }
    }
    
    fn main() {
        let pos = classify(10);
        let neg = classify(-10);
        let zero = classify(0);
    }
    "#;
    
    let ir = assert_compile_success(source, "multi_return");
    assert_valid_ir(&ir);
    
    // 複数の戻り値パスが含まれていることを確認
    assert!(ir.matches("ret i32").count() >= 3, "Should contain multiple return statements");
    assert!(ir.contains("icmp"), "Should contain comparisons");
    assert!(ir.contains("br"), "Should contain branches");
}

#[test]
fn test_recursive_functions_codegen() {
    // 再帰関数のコード生成テスト
    let source = r#"
    package main
    
    fn factorial(n: i32): i32 {
        if n <= 1 {
            return 1;
        } else {
            return n * factorial(n - 1);
        }
    }
    
    fn main() {
        let result = factorial(5);
    }
    "#;
    
    let ir = assert_compile_success(source, "recursive");
    assert_valid_ir(&ir);
    
    // 再帰呼び出しが含まれていることを確認
    assert!(ir.contains("call"), "Should contain recursive function call");
}

#[test]
fn test_function_calls_codegen() {
    // 関数呼び出しのコード生成テスト
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    
    fn main() {
        let result = add(5, 3);
        println("Result:", result);
    }
    "#;
    
    let ir = assert_compile_success(source, "functions");
    assert_valid_ir(&ir);
    
    // 関数定義と呼び出しが含まれていることを確認
    assert!(ir.contains("define") && ir.matches("define").count() >= 2, "Should contain multiple function definitions");
    assert!(ir.contains("call"), "Should contain function call");
}