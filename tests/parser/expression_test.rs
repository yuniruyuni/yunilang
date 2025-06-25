//! 式のパーサーテスト

use super::*;

#[test]
fn test_arithmetic_expressions() {
    // 算術式の解析テスト
    let source = r#"
    package main
    
    fn main() {
        let a = 1 + 2 * 3;
        let b = (4 + 5) * 6;
        let c = 7 - 8 / 2;
        let d = 9 % 10;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 4);
        
        // すべて変数宣言文であることを確認
        for stmt in &func.body.statements {
            assert!(matches!(stmt, Statement::Let(_)));
        }
    }
}

#[test]
fn test_boolean_expressions() {
    // ブール式の解析テスト
    let source = r#"
    package main
    
    fn main() {
        let a = true && false;
        let b = x > 5 || y < 10;
        let c = !flag;
        let d = x == y && z != w;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 4);
    }
}

#[test]
fn test_operator_precedence() {
    // 演算子優先順位の解析テスト
    let source = r#"
    package main
    
    fn main() {
        let a = 1 + 2 * 3;          // 1 + (2 * 3) = 7
        let b = 2 * 3 + 4;          // (2 * 3) + 4 = 10
        let c = 1 + 2 * 3 + 4;      // 1 + (2 * 3) + 4 = 11
        let d = 2 * 3 * 4;          // (2 * 3) * 4 = 24
        let e = 8 / 2 / 2;          // (8 / 2) / 2 = 2
        let f = a && b || c;        // (a && b) || c
        let g = !a && b;            // (!a) && b
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 7);
    }
}

#[test]
fn test_nested_expressions() {
    // ネストした式の解析テスト
    let source = r#"
    package main
    
    fn main() {
        let result = ((1 + 2) * (3 + 4)) / ((5 - 2) + (8 / 2));
        let complex = func1(func2(a, b), func3(c, func4(d, e)));
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 2);
    }
}