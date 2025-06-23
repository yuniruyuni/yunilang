//! 借用チェッカーのテスト

use yunilang::analyzer::SemanticAnalyzer;
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;

#[test]
fn test_use_after_move() {
    let source = r#"
        package test

        fn test_move() {
            let x = "hello";
            let y = x;  // xが移動される
            let z = x;  // エラー：移動後の使用
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    
    // 借用チェックエラーが発生することを確認
    assert!(result.is_err(), "移動後の使用がエラーにならなかった");
}

#[test]
fn test_multiple_mutable_borrows() {
    let source = r#"
        package test

        fn test_multiple_borrows() {
            let mut x = 42;
            let y = &mut x;  // 可変借用
            let z = &mut x;  // エラー：複数の可変借用
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    
    // 借用チェックエラーが発生することを確認
    assert!(result.is_err(), "複数の可変借用がエラーにならなかった");
}

#[test]
fn test_immutable_variable_assignment() {
    let source = r#"
        package test

        fn test_immutable() {
            let x = 42;     // 不変変数
            x = 100;        // エラー：不変変数への代入
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    
    // 借用チェックエラーが発生することを確認
    assert!(result.is_err(), "不変変数への代入がエラーにならなかった");
}

#[test]
fn test_valid_shared_borrows() {
    let source = r#"
        package test

        fn test_valid_borrows() {
            // 共有借用は複数可能
            let x = 42;
            let y = &x;
            let z = &x;
            let a = *y;
            let b = *z;
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    
    // 有効な借用パターンはエラーにならない
    assert!(result.is_ok(), "有効な借用パターンでエラーが発生: {:?}", result);
}

#[test]
fn test_move_while_borrowed() {
    let source = r#"
        package test

        fn test_move_while_borrowed() {
            let x = "hello";
            let y = &x;     // xを借用
            let z = x;      // エラー：借用中の移動
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("パースに失敗");
    
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(&program);
    
    // 借用チェックエラーが発生することを確認
    assert!(result.is_err(), "借用中の移動がエラーにならなかった");
}