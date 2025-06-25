//! 関数定義のパーサーテスト

use super::*;

#[test]
fn test_function_with_parameters() {
    // パラメータを持つ関数の解析テスト
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        
        assert_eq!(func.params[0].name, "a");
        assert!(matches!(func.params[0].ty, Type::I32));
        
        assert_eq!(func.params[1].name, "b");
        assert!(matches!(func.params[1].ty, Type::I32));
        
        assert!(func.return_type.is_some());
        if let Some(ref ret_type) = func.return_type {
            assert!(matches!(**ret_type, Type::I32));
        }
    }
}

#[test]
fn test_empty_function_parameters() {
    // 空の関数パラメータリストは有効
    let source = r#"
    package main
    
    fn test() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.params.len(), 0);
    }
}

#[test]
fn test_multiple_functions() {
    // 複数の関数定義
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    
    fn subtract(a: i32, b: i32): i32 {
        return a - b;
    }
    
    fn main() {
        let result1 = add(5, 3);
        let result2 = subtract(10, 4);
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    assert_eq!(ast.items.len(), 3);
    
    for item in &ast.items {
        assert!(matches!(item, Item::Function(_)));
    }
}