//! match式のコード生成テスト

use yunilang::ast::*;
use yunilang::codegen::CodeGenerator;
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;
use yunilang::analyzer::SemanticAnalyzer;
use yunilang::error::YuniResult;
use inkwell::context::Context;

/// マッチ式の基本的なコード生成をテスト
#[test]
fn test_match_expr_basic() -> YuniResult<()> {
    let context = Context::create();
    let mut generator = CodeGenerator::new(&context, "test_module");
    
    // enum宣言を手動で登録（CodeGeneratorには直接的なメソッドがないため）
    // Enumバリアントを登録
    generator.enum_variants.insert(("Option".to_string(), "Some".to_string()), 0);
    generator.enum_variants.insert(("Option".to_string(), "None".to_string()), 1);
    generator.type_manager.register_enum("Option".to_string(), generator.context.i32_type());
    
    // main関数を作成
    let main_fn_type = generator.context.i32_type().fn_type(&[], false);
    let main_fn = generator.module.add_function("main", main_fn_type, None);
    let main_bb = generator.context.append_basic_block(main_fn, "entry");
    generator.builder.position_at_end(main_bb);
    
    // Option::Some(Unit)を作成
    let some_variant = Expression::EnumVariant(EnumVariantExpr {
        enum_name: "Option".to_string(),
        variant: "Some".to_string(),
        fields: EnumVariantFields::Unit,
        span: Span { start: 0, end: 0 },
    });
    
    // match式を作成
    let match_expr = Expression::Match(MatchExpr {
        expr: Box::new(some_variant),
        arms: vec![
            MatchArm {
                pattern: Pattern::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "Some".to_string(),
                    fields: EnumVariantPatternFields::Unit,
                },
                guard: None,
                expr: Expression::Integer(IntegerLit {
                    value: 1,
                    suffix: None,
                    span: Span { start: 0, end: 0 },
                }),
            },
            MatchArm {
                pattern: Pattern::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "None".to_string(),
                    fields: EnumVariantPatternFields::Unit,
                },
                guard: None,
                expr: Expression::Integer(IntegerLit {
                    value: 0,
                    suffix: None,
                    span: Span { start: 0, end: 0 },
                }),
            },
        ],
        span: Span { start: 0, end: 0 },
    });
    
    // match式をコンパイル
    let result = generator.compile_expression(&match_expr)?;
    
    // 結果を返す
    generator.builder.build_return(Some(&result))?;
    
    // 生成されたLLVM IRを検証
    generator.module.verify().map_err(|e| {
        yunilang::error::YuniError::Codegen(yunilang::error::CodegenError::Internal {
            message: format!("Module verification failed: {}", e),
        })
    })?;
    
    Ok(())
}

/// 識別子パターンを使用したマッチ式のテスト
#[test]
fn test_match_expr_with_identifier_pattern() -> YuniResult<()> {
    let context = Context::create();
    let mut generator = CodeGenerator::new(&context, "test_module");
    
    // main関数を作成
    let main_fn_type = generator.context.i32_type().fn_type(&[], false);
    let main_fn = generator.module.add_function("main", main_fn_type, None);
    let main_bb = generator.context.append_basic_block(main_fn, "entry");
    generator.builder.position_at_end(main_bb);
    
    // 整数値42を作成
    let value = Expression::Integer(IntegerLit {
        value: 42,
        suffix: None,
        span: Span { start: 0, end: 0 },
    });
    
    // match式を作成（識別子パターンでバインド）
    let match_expr = Expression::Match(MatchExpr {
        expr: Box::new(value),
        arms: vec![
            MatchArm {
                pattern: Pattern::Identifier("x".to_string(), false),
                guard: None,
                expr: Expression::Binary(BinaryExpr {
                    left: Box::new(Expression::Identifier(Identifier {
                        name: "x".to_string(),
                        span: Span { start: 0, end: 0 },
                    })),
                    op: BinaryOp::Add,
                    right: Box::new(Expression::Integer(IntegerLit {
                        value: 1,
                        suffix: None,
                        span: Span { start: 0, end: 0 },
                    })),
                    span: Span { start: 0, end: 0 },
                }),
            },
        ],
        span: Span { start: 0, end: 0 },
    });
    
    // match式をコンパイル
    let result = generator.compile_expression(&match_expr)?;
    
    // 結果を返す
    generator.builder.build_return(Some(&result))?;
    
    // 生成されたLLVM IRを検証
    generator.module.verify().map_err(|e| {
        yunilang::error::YuniError::Codegen(yunilang::error::CodegenError::Internal {
            message: format!("Module verification failed: {}", e),
        })
    })?;
    
    Ok(())
}

/// ガード付きマッチ式のテスト
#[test]
fn test_match_expr_with_guard() -> YuniResult<()> {
    let context = Context::create();
    let mut generator = CodeGenerator::new(&context, "test_module");
    
    // main関数を作成
    let main_fn_type = generator.context.i32_type().fn_type(&[], false);
    let main_fn = generator.module.add_function("main", main_fn_type, None);
    let main_bb = generator.context.append_basic_block(main_fn, "entry");
    generator.builder.position_at_end(main_bb);
    
    // 整数値10を作成
    let value = Expression::Integer(IntegerLit {
        value: 10,
        suffix: None,
        span: Span { start: 0, end: 0 },
    });
    
    // match式を作成（ガード付き）
    let match_expr = Expression::Match(MatchExpr {
        expr: Box::new(value),
        arms: vec![
            MatchArm {
                pattern: Pattern::Identifier("x".to_string(), false),
                guard: Some(Expression::Binary(BinaryExpr {
                    left: Box::new(Expression::Identifier(Identifier {
                        name: "x".to_string(),
                        span: Span { start: 0, end: 0 },
                    })),
                    op: BinaryOp::Gt,
                    right: Box::new(Expression::Integer(IntegerLit {
                        value: 5,
                        suffix: None,
                        span: Span { start: 0, end: 0 },
                    })),
                    span: Span { start: 0, end: 0 },
                })),
                expr: Expression::Integer(IntegerLit {
                    value: 1,
                    suffix: None,
                    span: Span { start: 0, end: 0 },
                }),
            },
            MatchArm {
                pattern: Pattern::Identifier("_".to_string(), false),
                guard: None,
                expr: Expression::Integer(IntegerLit {
                    value: 0,
                    suffix: None,
                    span: Span { start: 0, end: 0 },
                }),
            },
        ],
        span: Span { start: 0, end: 0 },
    });
    
    // match式をコンパイル
    let result = generator.compile_expression(&match_expr)?;
    
    // 結果を返す
    generator.builder.build_return(Some(&result))?;
    
    // 生成されたLLVM IRを検証
    generator.module.verify().map_err(|e| {
        yunilang::error::YuniError::Codegen(yunilang::error::CodegenError::Internal {
            message: format!("Module verification failed: {}", e),
        })
    })?;
    
    Ok(())
}

/// パーサーからASTを生成してmatch式をコンパイルするテスト
/// TODO: パーサーがmatch式をサポートした後に有効化する
#[test]
#[ignore = "Parser does not yet support match expressions"]
fn test_match_expr_from_parser() -> YuniResult<()> {
    let source = r#"
package test

type Result enum {
    Ok {},
    Err {},
}

fn main() -> i32 {
    let x = Result::Ok;
    match x {
        Result::Ok => { 0 },
        Result::Err => { 1 },
    }
}
    "#;
    
    // レクサーとパーサーでASTを生成
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    // セマンティック解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;
    
    // コード生成
    let context = Context::create();
    let mut generator = CodeGenerator::new(&context, "test_module");
    generator.compile_program(&ast)?;
    
    // 生成されたLLVM IRを検証
    generator.module.verify().map_err(|e| {
        yunilang::error::YuniError::Codegen(yunilang::error::CodegenError::Internal {
            message: format!("Module verification failed: {}", e),
        })
    })?;
    
    Ok(())
}