//! テンプレート文字列のパーサーテスト

use super::*;

#[test]
fn test_template_string_parsing() {
    // 簡単なテンプレート文字列
    let source = r#"
    package main
    
    fn main() {
        let name = "Yuni";
        let age = 42;
        let msg = `Hello, ${name}! You are ${age} years old.`;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        if let Statement::Let(ref let_stmt) = func.body.statements[2] {
            if let Some(Expression::TemplateString(ref template)) = &let_stmt.init {
                assert_eq!(template.parts.len(), 5);
                
                // "Hello, "
                if let TemplateStringPart::Text(text) = &template.parts[0] {
                    assert_eq!(text, "Hello, ");
                } else {
                    panic!("Expected text part");
                }
                
                // ${name}
                if let TemplateStringPart::Interpolation(Expression::Identifier(ref id)) = &template.parts[1] {
                    assert_eq!(id.name, "name");
                } else {
                    panic!("Expected interpolation with identifier");
                }
                
                // "! You are "
                if let TemplateStringPart::Text(text) = &template.parts[2] {
                    assert_eq!(text, "! You are ");
                } else {
                    panic!("Expected text part");
                }
                
                // ${age}
                if let TemplateStringPart::Interpolation(Expression::Identifier(ref id)) = &template.parts[3] {
                    assert_eq!(id.name, "age");
                } else {
                    panic!("Expected interpolation with identifier");
                }
                
                // " years old."
                if let TemplateStringPart::Text(text) = &template.parts[4] {
                    assert_eq!(text, " years old.");
                } else {
                    panic!("Expected text part");
                }
            } else {
                panic!("Expected template string");
            }
        } else {
            panic!("Expected let statement");
        }
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_template_string_with_expressions() {
    // 式を含むテンプレート文字列
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        let y = 20;
        let result = `The sum of ${x} and ${y} is ${x + y}`;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        if let Statement::Let(ref let_stmt) = func.body.statements[2] {
            if let Some(Expression::TemplateString(ref template)) = &let_stmt.init {
                assert_eq!(template.parts.len(), 6);
                
                // 最後の補間式が x + y であることを確認
                if let TemplateStringPart::Interpolation(Expression::Binary(ref binary)) = &template.parts[5] {
                    assert_eq!(binary.op, BinaryOp::Add);
                    if let Expression::Identifier(ref left) = binary.left.as_ref() {
                        assert_eq!(left.name, "x");
                    } else {
                        panic!("Expected identifier x");
                    }
                    if let Expression::Identifier(ref right) = binary.right.as_ref() {
                        assert_eq!(right.name, "y");
                    } else {
                        panic!("Expected identifier y");
                    }
                } else {
                    panic!("Expected binary expression");
                }
            } else {
                panic!("Expected template string");
            }
        } else {
            panic!("Expected let statement");
        }
    } else {
        panic!("Expected function");
    }
}