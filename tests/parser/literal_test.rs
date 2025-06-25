//! リテラルのパーサーテスト

use super::*;

#[test]
fn test_integer_literal_with_suffix() {
    // 型サフィックス付き整数リテラルのテスト
    let source = r#"
    package main
    
    fn main() {
        let a = 42i32;
        let b = 100u64;
        let c = 255u8;
        let d = -128i8;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 4);
        
        // 42i32
        if let Statement::Let(ref let_stmt) = func.body.statements[0] {
            if let Some(Expression::Integer(ref int_lit)) = &let_stmt.init {
                assert_eq!(int_lit.value, 42);
                assert_eq!(int_lit.suffix, Some("i32".to_string()));
            } else {
                panic!("Expected integer literal");
            }
        }
        
        // 100u64
        if let Statement::Let(ref let_stmt) = func.body.statements[1] {
            if let Some(Expression::Integer(ref int_lit)) = &let_stmt.init {
                assert_eq!(int_lit.value, 100);
                assert_eq!(int_lit.suffix, Some("u64".to_string()));
            } else {
                panic!("Expected integer literal");
            }
        }
        
        // 255u8
        if let Statement::Let(ref let_stmt) = func.body.statements[2] {
            if let Some(Expression::Integer(ref int_lit)) = &let_stmt.init {
                assert_eq!(int_lit.value, 255);
                assert_eq!(int_lit.suffix, Some("u8".to_string()));
            } else {
                panic!("Expected integer literal");
            }
        }
        
        // -128i8
        if let Statement::Let(ref let_stmt) = func.body.statements[3] {
            if let Some(Expression::Integer(ref int_lit)) = &let_stmt.init {
                assert_eq!(int_lit.value, -128);
                assert_eq!(int_lit.suffix, Some("i8".to_string()));
            } else {
                panic!("Expected integer literal");
            }
        }
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_float_literal_with_suffix() {
    // 型サフィックス付き浮動小数点リテラルのテスト
    let source = r#"
    package main
    
    fn main() {
        let a = 4.14f32;
        let b = 3.71828f64;
        let c = 0.5f32;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 3);
        
        // 3.14f32
        if let Statement::Let(ref let_stmt) = func.body.statements[0] {
            if let Some(Expression::Float(ref float_lit)) = &let_stmt.init {
                assert!((float_lit.value - 4.14).abs() < 0.001);
                assert_eq!(float_lit.suffix, Some("f32".to_string()));
            } else {
                panic!("Expected float literal");
            }
        }
        
        // 2.71828f64
        if let Statement::Let(ref let_stmt) = func.body.statements[1] {
            if let Some(Expression::Float(ref float_lit)) = &let_stmt.init {
                assert!((float_lit.value - 3.71828).abs() < 0.000001);
                assert_eq!(float_lit.suffix, Some("f64".to_string()));
            } else {
                panic!("Expected float literal");
            }
        }
        
        // 0.5f32
        if let Statement::Let(ref let_stmt) = func.body.statements[2] {
            if let Some(Expression::Float(ref float_lit)) = &let_stmt.init {
                assert!((float_lit.value - 0.5).abs() < 0.001);
                assert_eq!(float_lit.suffix, Some("f32".to_string()));
            } else {
                panic!("Expected float literal");
            }
        }
    } else {
        panic!("Expected function");
    }
}