//! 文のパーサーテスト

use super::*;

#[test]
fn test_variable_declarations() {
    // 変数宣言の解析テスト
    let source = r#"
    package main
    
    fn main() {
        let x: i32 = 42;
        let mut y: f64 = 3.14;
        let z = "hello";
        let flag = true; // true/falseはToken::True/Token::Falseとして認識される
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 4);
        
        // let x: i32 = 42;
        if let Statement::Let(ref let_stmt) = func.body.statements[0] {
            if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                assert_eq!(name, "x");
                assert!(!is_mut);
            }
            assert!(let_stmt.ty.is_some());
            assert!(let_stmt.init.is_some());
        }
        
        // let mut y: f64 = 3.14;
        if let Statement::Let(ref let_stmt) = func.body.statements[1] {
            if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                assert_eq!(name, "y");
                assert!(is_mut);
            }
            assert!(let_stmt.ty.is_some());
            assert!(let_stmt.init.is_some());
        }
        
        // let z = "hello"; (型推論)
        if let Statement::Let(ref let_stmt) = func.body.statements[2] {
            if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                assert_eq!(name, "z");
                assert!(!is_mut);
            }
            assert!(let_stmt.ty.is_none());
            assert!(let_stmt.init.is_some());
        }
    }
}

#[test]
fn test_return_statements() {
    // return文の解析テスト
    let source = r#"
    package main
    
    fn get_zero(): i32 {
        return 0;
    }
    
    fn get_nothing() {
        return;
    }
    
    fn early_return(x: i32): i32 {
        if x < 0 {
            return -1;
        }
        return x * 2;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    assert_eq!(ast.items.len(), 3);
    
    // すべて関数であることを確認
    for item in &ast.items {
        assert!(matches!(item, Item::Function(_)));
    }
}

#[test]
fn test_function_calls() {
    // 関数呼び出しの解析テスト
    let source = r#"
    package main
    
    fn main() {
        println();
        println("hello");
        println("x =", x);
        let result = add(1, 2);
        let nested = add(mul(3, 4), div(8, 2));
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 5);
    }
}