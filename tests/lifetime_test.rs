//! ライフタイムパラメータのテスト

use yunilang::analyzer::SemanticAnalyzer;
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;

#[test]
fn test_lifetime_parameters_processing() {
    // 簡単な関数で実装のテスト
    let source = r#"
        package test

        fn longest(x: &str, y: &str): &str {
            return x;
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    // ライフタイム句を手動で追加してテスト
    if let yunilang::ast::Item::Function(ref mut func) = program.items[0].clone() {
        func.lives_clause = Some(yunilang::ast::LivesClause {
            constraints: vec![
                yunilang::ast::LivesConstraint {
                    target: "'a".to_string(),
                    sources: vec!["'b".to_string()],
                    span: yunilang::ast::Span::dummy(),
                }
            ],
            span: yunilang::ast::Span::dummy(),
        });
        
        let mut analyzer = SemanticAnalyzer::new();
        // 手動で関数を解析
        let result = analyzer.analyze_function(&func);
        
        // ライフタイムパラメータがエラーなく処理されることを確認
        assert!(result.is_ok(), "ライフタイム解析でエラーが発生: {:?}", result);
        
        // ライフタイムコンテキストに登録されていることを確認
        assert!(analyzer.lifetime_context.lifetimes.len() > 1); // 最低でも'staticと'a、'bがある
        assert!(!analyzer.lifetime_context.constraints.is_empty()); // 制約が登録されている
    } else {
        panic!("関数が見つかりませんでした");
    }
}

#[test] 
fn test_multiple_lifetime_constraints() {
    // 複数のライフタイム制約のテスト
    let source = r#"
        package test

        fn complex_lifetime(x: &str, y: &str, z: &str, w: &str): &str {
            return w;
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    // ライフタイム句を手動で追加してテスト
    if let yunilang::ast::Item::Function(ref mut func) = program.items[0].clone() {
        func.lives_clause = Some(yunilang::ast::LivesClause {
            constraints: vec![
                yunilang::ast::LivesConstraint {
                    target: "'a".to_string(),
                    sources: vec!["'b".to_string()],
                    span: yunilang::ast::Span::dummy(),
                },
                yunilang::ast::LivesConstraint {
                    target: "'c".to_string(),
                    sources: vec!["'d".to_string()],
                    span: yunilang::ast::Span::dummy(),
                },
                yunilang::ast::LivesConstraint {
                    target: "'b".to_string(),
                    sources: vec!["'c".to_string()],
                    span: yunilang::ast::Span::dummy(),
                }
            ],
            span: yunilang::ast::Span::dummy(),
        });
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_function(&func);
        
        // 複数のライフタイム制約がエラーなく処理されることを確認
        assert!(result.is_ok(), "複数のライフタイム制約の解析でエラーが発生: {:?}", result);
        
        // すべての制約が登録されていることを確認
        assert!(analyzer.lifetime_context.constraints.len() >= 3); // 'a: 'b, 'c: 'd, 'b: 'c
    }
}

#[test]
fn test_lifetime_parameters_in_method() {
    // メソッドのライフタイムパラメータ処理のテスト
    let method = yunilang::ast::MethodDecl {
        is_public: false,
        name: "concat".to_string(),
        receiver: yunilang::ast::Receiver {
            name: Some("self".to_string()),
            ty: yunilang::ast::Type::Reference(
                Box::new(yunilang::ast::Type::UserDefined("StringHolder".to_string())),
                false
            ),
            is_mut: false,
            span: yunilang::ast::Span::dummy(),
        },
        params: vec![
            yunilang::ast::Param {
                name: "other".to_string(),
                ty: yunilang::ast::Type::Reference(Box::new(yunilang::ast::Type::Str), false),
                span: yunilang::ast::Span::dummy(),
            }
        ],
        return_type: Some(Box::new(yunilang::ast::Type::Str)),
        lives_clause: Some(yunilang::ast::LivesClause {
            constraints: vec![
                yunilang::ast::LivesConstraint {
                    target: "'a".to_string(),
                    sources: vec!["'b".to_string()],
                    span: yunilang::ast::Span::dummy(),
                }
            ],
            span: yunilang::ast::Span::dummy(),
        }),
        body: yunilang::ast::Block {
            statements: vec![
                yunilang::ast::Statement::Return(yunilang::ast::ReturnStatement {
                    value: Some(yunilang::ast::Expression::Identifier(yunilang::ast::Identifier {
                        name: "other".to_string(),
                        span: yunilang::ast::Span::dummy(),
                    })),
                    span: yunilang::ast::Span::dummy(),
                })
            ],
            span: yunilang::ast::Span::dummy(),
        },
        span: yunilang::ast::Span::dummy(),
    };
    
    let mut analyzer = SemanticAnalyzer::new();
    // 新しいスコープを作成
    analyzer.enter_scope();
    
    let result = analyzer.analyze_method(&method);
    
    // メソッドのライフタイムパラメータがエラーなく処理されることを確認
    assert!(result.is_ok(), "メソッドのライフタイム解析でエラーが発生: {:?}", result);
    
    // ライフタイムコンテキストに登録されていることを確認
    assert!(analyzer.lifetime_context.lifetimes.len() > 1); // 最低でも'staticと'a、'bがある
    assert!(!analyzer.lifetime_context.constraints.is_empty()); // 制約が登録されている
}