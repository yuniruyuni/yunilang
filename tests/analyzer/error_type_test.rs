//! 型エラーのセマンティック解析テスト

use super::*;

#[test]
fn test_type_mismatch_error() {
    // 型不一致エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x: i32 = "hello";  // 型不一致
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}

#[test]
fn test_immutable_assignment_error() {
    // 不変変数への代入エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        x = 20;  // 不変変数への代入
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::ImmutableVariable { .. })
    });
}

#[test]
fn test_return_type_mismatch_error() {
    // 戻り値型不一致エラーのテスト
    let source = r#"
    package main
    
    fn get_number(): i32 {
        return "hello";  // 型不一致
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}

#[test]
fn test_missing_return_error() {
    // 戻り値不足エラーのテスト
    let source = r#"
    package main
    
    fn get_number(): i32 {
        let x = 42;
        // return文がない
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::MissingReturn { .. })
    });
}

#[test]
fn test_arithmetic_type_mismatch_error() {
    // 算術演算型不一致エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x: i32 = 10;
        let y: f64 = 3.14;
        let result = x + y;  // 型が不一致
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}

#[test]
fn test_comparison_type_mismatch_error() {
    // 比較演算型不一致エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        let y = "hello";
        let result = x == y;  // 型が不一致
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}

#[test]
fn test_logical_operation_type_error() {
    // 論理演算型エラーのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        let result = x && true;  // i32はboolではない
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}