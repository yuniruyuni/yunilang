//! 可視性修飾子のパーサーテスト

use super::*;

#[test]
fn test_visibility_modifiers() {
    // 公開関数のテスト
    let source = r#"
    package main
    
    pub fn public_function() {
    }
    
    fn private_function() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    assert_eq!(ast.items.len(), 2);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.name, "public_function");
        assert!(func.is_public);
    } else {
        panic!("Expected function");
    }
    
    if let Item::Function(ref func) = ast.items[1] {
        assert_eq!(func.name, "private_function");
        assert!(!func.is_public);
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_visibility_modifiers_on_methods() {
    // 公開メソッドのテスト
    let source = r#"
package main

type Point struct {
    x: i32,
    y: i32
}

pub impl fn add(p1: &Point, p2: &Point): Point {
    return Point { x: p1.x + p2.x, y: p1.y + p2.y };
}

impl fn private_method(p: &Point) {
}
"#;
    
    let ast = assert_parse_success(source);
    assert_eq!(ast.items.len(), 3);
    
    if let Item::Method(ref method) = ast.items[1] {
        assert_eq!(method.name, "add");
        assert!(method.is_public);
    } else {
        panic!("Expected method");
    }
    
    if let Item::Method(ref method) = ast.items[2] {
        assert_eq!(method.name, "private_method");
        assert!(!method.is_public);
    } else {
        panic!("Expected method");
    }
}

#[test]
fn test_visibility_modifiers_errors() {
    // 構造体に可視性修飾子を付けるとエラー
    assert_parse_error("package main\npub type Point struct { x: i32, }");
    
    // enumに可視性修飾子を付けるとエラー
    assert_parse_error("package main\npub type Option enum { Some, None }");
    
    // 型定義に可視性修飾子を付けるとエラー
    assert_parse_error("package main\npub type MyInt = i32;");
}