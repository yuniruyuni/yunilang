//! テンプレート文字列の高度なパーサーテスト

use super::*;

#[test]
fn test_template_string_escape_sequences() {
    // エスケープシーケンスを含むテンプレート文字列
    let source = r#"
    package main
    
    fn main() {
        let name = "World";
        let msg = `Hello\n\t${name}!\nSpecial chars: \` \$ \\`;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        if let Statement::Let(ref let_stmt) = func.body.statements[1] {
            if let Some(Expression::TemplateString(ref template)) = &let_stmt.init {
                assert_eq!(template.parts.len(), 3);
                
                // "Hello\n\t"
                if let TemplateStringPart::Text(text) = &template.parts[0] {
                    assert_eq!(text, "Hello\n\t");
                } else {
                    panic!("Expected text part");
                }
                
                // "!\nSpecial chars: ` $ \\"
                if let TemplateStringPart::Text(text) = &template.parts[2] {
                    assert_eq!(text, "!\nSpecial chars: ` $ \\");
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
fn test_template_string_empty() {
    // 空のテンプレート文字列
    let source = r#"
    package main
    
    fn main() {
        let msg = ``;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        if let Statement::Let(ref let_stmt) = func.body.statements[0] {
            if let Some(Expression::TemplateString(ref template)) = &let_stmt.init {
                assert_eq!(template.parts.len(), 0);
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
fn test_template_string_nested_braces() {
    // ネストされた括弧を含むテンプレート文字列
    let source = r#"
    package main
    
    fn main() {
        let obj = MyStruct { x: 10 };
        let msg = `Object: ${obj}`;
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        if let Statement::Let(ref let_stmt) = func.body.statements[1] {
            if let Some(Expression::TemplateString(ref template)) = &let_stmt.init {
                assert_eq!(template.parts.len(), 2);
                
                // "Object: "
                if let TemplateStringPart::Text(text) = &template.parts[0] {
                    assert_eq!(text, "Object: ");
                } else {
                    panic!("Expected text part");
                }
                
                // ${obj}
                if let TemplateStringPart::Interpolation(Expression::Identifier(ref id)) = &template.parts[1] {
                    assert_eq!(id.name, "obj");
                } else {
                    panic!("Expected interpolation with identifier");
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