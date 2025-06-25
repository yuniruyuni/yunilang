//! 変数・関数エラーのセマンティック解析テスト

use super::*;

#[test]
fn test_undefined_variable_error() {
    // 未定義変数エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = y + 1;  // yが未定義
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UndefinedVariable { .. })
    });
}

#[test]
fn test_undefined_function_error() {
    // 未定義関数エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let result = unknown_function(42);  // 未定義関数
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UndefinedFunction { .. })
    });
}

#[test]
fn test_undefined_type_error() {
    // 未定義型エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x: UnknownType = 42;  // 未定義型
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UndefinedType { .. })
    });
}

#[test]
fn test_duplicate_variable_error() {
    // 重複変数エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        let x = 20;  // 同じスコープで重複
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::DuplicateVariable { .. })
    });
}

#[test]
fn test_duplicate_function_error() {
    // 重複関数エラーのテスト
    let source = r#"
    package main
    
    fn test() {
    }
    
    fn test() {  // 重複関数
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::DuplicateFunction { .. })
    });
}

#[test]
fn test_argument_count_mismatch_error() {
    // 引数数不一致エラーのテスト
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    
    fn main() {
        let result = add(5);  // 引数が不足
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::ArgumentCountMismatch { .. })
    });
}

#[test]
fn test_function_overloading_error() {
    // 関数オーバーロードエラーのテスト（Yuniでは未サポート）
    let source = r#"
    package main
    
    fn process(x: i32) {
        println(x);
    }
    
    fn process(x: f64) {  // 同名関数（オーバーロード）
        println(x);
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::DuplicateFunction { .. })
    });
}