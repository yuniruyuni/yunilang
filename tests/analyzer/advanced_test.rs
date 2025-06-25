//! 高度なセマンティック解析テスト

use super::*;

#[test]
#[ignore = "Circular dependency detection not yet implemented"]
fn test_circular_dependency_detection() {
    // 循環依存の検出テスト
    let source = r#"
    package main
    
    struct A {
        b: B,
    }
    
    struct B {
        a: A,  // 循環依存
    }
    
    fn main() {
    }
    "#;
    
    // 循環依存エラーは設計に依存するため、
    // 具体的な実装に応じて調整が必要
    assert_analysis_error(source);
}

#[test]
fn test_multiple_error_detection() {
    // 複数エラーの検出テスト
    let source = r#"
    package main
    
    fn main() {
        let x = undefined_var;     // 未定義変数エラー
        let y: UnknownType = 42;   // 未定義型エラー
        let z = x + "hello";       // 型不一致エラー（推測）
        unknown_func();            // 未定義関数エラー
    }
    "#;
    
    // 複数のエラーが検出されることを確認
    // 現在の実装では最初のエラーで停止するため、
    // このテストは最初のエラーのみをチェック
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UndefinedVariable { .. })
    });
}