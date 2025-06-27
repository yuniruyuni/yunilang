//! 制御フローのパーサーテスト

use super::*;

#[test]
fn test_if_statements() {
    // if文の解析テスト
    let source = r#"
    package main
    
    fn main() {
        if x > 0 {
            println("positive");
        }
        
        if y < 0 {
            println("negative");
        } else {
            println("non-negative");
        }
        
        if z == 0 {
            println("zero");
        } else if z > 0 {
            println("positive");
        } else {
            println("negative");
        }
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 3);
        
        // すべてif文であることを確認
        for stmt in &func.body.statements {
            assert!(matches!(stmt, Statement::If(_)));
        }
    }
}

#[test]
fn test_while_loops() {
    // while文の解析テスト
    let source = r#"
    package main
    
    fn main() {
        while x > 0 {
            x = x - 1;
        }
        
        let mut i = 0;
        while i < 10 {
            println(i);
            i = i + 1;
        }
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        // while文が含まれていることを確認
        let has_while = func.body.statements.iter().any(|stmt| {
            matches!(stmt, Statement::While(_))
        });
        assert!(has_while);
    }
}

#[test]
fn test_for_loops() {
    // for文の解析テスト
    let source = r#"
    package main
    
    fn main() {
        // 標準的なfor文
        for let mut i = 0; i < 10; i = i + 1 {
            println(i);
        }
        
        // 初期化部を省略したfor文
        let mut j = 0;
        for ; j < 5; j = j + 1 {
            println(j);
        }
        
        // 無限ループ
        for {
            println("infinite loop");
            break;
        }
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        // for文が含まれていることを確認
        let has_for = func.body.statements.iter().any(|stmt| {
            matches!(stmt, Statement::For(_))
        });
        assert!(has_for, "Should contain for statements");
    }
}