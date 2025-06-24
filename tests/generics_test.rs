//! ジェネリクスのテスト

use yunilang::parser::Parser;
use yunilang::ast::Type;
use yunilang::lexer::Lexer;

#[test]
fn test_parse_generic_struct() {
    let input = r#"
package test

struct Vec<T> {
    data: [T],
    len: u64,
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result);
    
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
    
    if let yunilang::ast::Item::TypeDef(yunilang::ast::TypeDef::Struct(struct_def)) = &program.items[0] {
        assert_eq!(struct_def.name, "Vec");
        assert_eq!(struct_def.type_params.len(), 1);
        assert_eq!(struct_def.type_params[0].name, "T");
        
        // dataフィールドの型をチェック
        assert_eq!(struct_def.fields.len(), 2);
        assert_eq!(struct_def.fields[0].name, "data");
        if let Type::Array(elem_type) = &struct_def.fields[0].ty {
            if let Type::TypeVariable(name) = elem_type.as_ref() {
                assert_eq!(name, "T");
            } else {
                panic!("Expected TypeVariable, got {:?}", elem_type);
            }
        } else {
            panic!("Expected Array type, got {:?}", struct_def.fields[0].ty);
        }
    } else {
        panic!("Expected struct definition");
    }
}

#[test]
fn test_parse_generic_function() {
    let input = r#"
package test

fn identity<T>(x: T): T {
    x
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result);
    
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
    
    if let yunilang::ast::Item::Function(func_decl) = &program.items[0] {
        assert_eq!(func_decl.name, "identity");
        assert_eq!(func_decl.type_params.len(), 1);
        assert_eq!(func_decl.type_params[0].name, "T");
        
        // パラメータの型をチェック
        assert_eq!(func_decl.params.len(), 1);
        assert_eq!(func_decl.params[0].name, "x");
        if let Type::TypeVariable(name) = &func_decl.params[0].ty {
            assert_eq!(name, "T");
        } else {
            panic!("Expected TypeVariable, got {:?}", func_decl.params[0].ty);
        }
        
        // 戻り値の型をチェック
        if let Some(return_type) = &func_decl.return_type {
            if let Type::TypeVariable(name) = return_type.as_ref() {
                assert_eq!(name, "T");
            } else {
                panic!("Expected TypeVariable, got {:?}", return_type);
            }
        } else {
            panic!("Expected return type");
        }
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_parse_generic_type_usage() {
    let input = r#"
package test

fn main() {
    let v: Vec<i32> = Vec::new();
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result);
    
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
    
    if let yunilang::ast::Item::Function(func_decl) = &program.items[0] {
        assert_eq!(func_decl.name, "main");
        
        // 最初の文を取得
        if let yunilang::ast::Statement::Let(let_stmt) = &func_decl.body.statements[0] {
            if let yunilang::ast::Pattern::Identifier(name, _) = &let_stmt.pattern {
                assert_eq!(name, "v");
            } else {
                panic!("Expected identifier pattern");
            }
            
            // 型注釈をチェック
            if let Some(type_annotation) = &let_stmt.ty {
                if let Type::Generic(name, args) = type_annotation {
                    assert_eq!(name, "Vec");
                    assert_eq!(args.len(), 1);
                    assert_eq!(args[0], Type::I32);
                } else {
                    panic!("Expected Generic type, got {:?}", type_annotation);
                }
            } else {
                panic!("Expected type annotation");
            }
        } else {
            panic!("Expected let statement");
        }
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_parse_multiple_type_params() {
    let input = r#"
package test

struct HashMap<K, V> {
    keys: [K],
    values: [V],
}
"#;

    let lexer = Lexer::new(input);
    let tokens = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse: {:?}", result);
    
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
    
    if let yunilang::ast::Item::TypeDef(yunilang::ast::TypeDef::Struct(struct_def)) = &program.items[0] {
        assert_eq!(struct_def.name, "HashMap");
        assert_eq!(struct_def.type_params.len(), 2);
        assert_eq!(struct_def.type_params[0].name, "K");
        assert_eq!(struct_def.type_params[1].name, "V");
        
        // フィールドの型をチェック
        assert_eq!(struct_def.fields.len(), 2);
        
        // keysフィールド
        assert_eq!(struct_def.fields[0].name, "keys");
        if let Type::Array(elem_type) = &struct_def.fields[0].ty {
            if let Type::TypeVariable(name) = elem_type.as_ref() {
                assert_eq!(name, "K");
            } else {
                panic!("Expected TypeVariable K, got {:?}", elem_type);
            }
        } else {
            panic!("Expected Array type, got {:?}", struct_def.fields[0].ty);
        }
        
        // valuesフィールド
        assert_eq!(struct_def.fields[1].name, "values");
        if let Type::Array(elem_type) = &struct_def.fields[1].ty {
            if let Type::TypeVariable(name) = elem_type.as_ref() {
                assert_eq!(name, "V");
            } else {
                panic!("Expected TypeVariable V, got {:?}", elem_type);
            }
        } else {
            panic!("Expected Array type, got {:?}", struct_def.fields[1].ty);
        }
    } else {
        panic!("Expected struct definition");
    }
}