//! 型定義のパーサーテスト

use super::*;

#[test]
fn test_struct_definition() {
    // 構造体定義の解析テスト
    let source = r#"
    package main
    
    type Point struct {
        x: f64,
        y: f64,
    }
    
    type Person struct {
        name: str,
        age: i32,
        active: bool,
    }
    
    fn main() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    // 構造体2つと関数1つ
    assert_eq!(ast.items.len(), 3);
    
    // 最初の2つは構造体
    assert!(matches!(ast.items[0], Item::TypeDef(TypeDef::Struct(_))));
    assert!(matches!(ast.items[1], Item::TypeDef(TypeDef::Struct(_))));
    
    if let Item::TypeDef(TypeDef::Struct(ref struct_def)) = ast.items[0] {
        assert_eq!(struct_def.name, "Point");
        assert_eq!(struct_def.fields.len(), 2);
    }
}

#[test]
fn test_enum_definition() {
    // 列挙型定義の解析テスト
    let source = r#"
    package main
    
    type Color enum {
        Red,
        Green,
        Blue,
    }
    
    type Option enum {
        Some { value: i32 },
        None,
    }
    
    fn main() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    // 列挙型2つと関数1つ
    assert_eq!(ast.items.len(), 3);
    
    // 最初の2つは列挙型
    assert!(matches!(ast.items[0], Item::TypeDef(TypeDef::Enum(_))));
    assert!(matches!(ast.items[1], Item::TypeDef(TypeDef::Enum(_))));
    
    if let Item::TypeDef(TypeDef::Enum(ref enum_def)) = ast.items[0] {
        assert_eq!(enum_def.name, "Color");
        assert_eq!(enum_def.variants.len(), 3);
    }
}

#[test]
fn test_type_alias() {
    // 型エイリアスのテスト
    let source = r#"
    package main
    
    type UserID i32
    type UserName String
    type Point2D struct { x: f64, y: f64 }
    
    fn main() {
        let id: UserID = 123;
        let name: UserName = "Alice";
    }
    "#;
    
    let ast = assert_parse_success(source);
    assert_eq!(ast.items.len(), 4); // 3 type defs + 1 function
    
    // UserID型エイリアス
    if let Item::TypeDef(TypeDef::Alias(ref alias)) = ast.items[0] {
        assert_eq!(alias.name, "UserID");
        assert!(matches!(alias.underlying_type, Type::I32));
    } else {
        panic!("Expected type alias");
    }
    
    // UserName型エイリアス
    if let Item::TypeDef(TypeDef::Alias(ref alias)) = ast.items[1] {
        assert_eq!(alias.name, "UserName");
        assert!(matches!(alias.underlying_type, Type::String));
    } else {
        panic!("Expected type alias");
    }
    
    // Point2D構造体
    if let Item::TypeDef(TypeDef::Struct(ref struct_def)) = ast.items[2] {
        assert_eq!(struct_def.name, "Point2D");
        assert_eq!(struct_def.fields.len(), 2);
    } else {
        panic!("Expected struct definition");
    }
}