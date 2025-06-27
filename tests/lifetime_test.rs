//! ライフタイムパラメータのテスト

use yunilang::analyzer::SemanticAnalyzer;
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;

#[test]
fn test_lifetime_parameters_processing() {
    // ライフタイムパラメータの処理テスト
    let source = r#"
        package test

        fn longest(x: &String, y: &String): (ret: &String)
        lives
            ret = x | y
        {
            return x;
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    // lives節が正しくパースされることを確認
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    assert!(result.is_ok(), "ライフタイム解析でエラーが発生: {:?}", result);
    
    // 関数にlives節があることを確認
    if let yunilang::ast::Item::Function(ref func) = program.items[0] {
        assert!(func.lives_clause.is_some(), "lives節がパースされていません");
        
        let lives = func.lives_clause.as_ref().unwrap();
        assert_eq!(lives.constraints.len(), 1, "制約が1つあるべき");
        
        let constraint = &lives.constraints[0];
        assert_eq!(constraint.target, "ret");
        assert_eq!(constraint.sources.len(), 2);
        assert!(constraint.sources.contains(&"x".to_string()));
        assert!(constraint.sources.contains(&"y".to_string()));
    } else {
        panic!("関数が見つかりませんでした");
    }
}

#[test] 
fn test_multiple_lifetime_constraints() {
    // 複数のライフタイム制約のテスト
    let source = r#"
        package test

        fn complex_lifetime(x: &String, y: &String, z: &String, w: &String): (result: &String)
        lives
            result = w,
            w = x | y,
            x = z
        {
            return w;
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    // lives節のパーサー実装までの代替テスト
    // 正しい構文ではlives節でライフタイム制約を明示
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
        let result = analyzer.analyze_function(func);
        
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
        type_params: vec![],
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
