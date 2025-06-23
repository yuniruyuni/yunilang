//! 統一的なエラーハンドリングのテスト

use yunilang::compiler::{CompilationPipeline, CompilationState};
use yunilang::error::{LexerError, ParserError, YuniError};

#[test]
fn test_lexer_error_handling() {
    // 不正なトークンを含むソースコード
    let source = r#"
fn main() {
    let x = @#$;  // 不正なトークン
}
"#;

    let state = CompilationState::new_from_string("test.yuni", source.to_string()).unwrap();
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // レキシカル解析を実行
    let tokens = pipeline.tokenize();
    
    // エラーが収集されていることを確認
    assert!(pipeline.state().has_errors());
    assert_eq!(pipeline.state().error_count(), 3); // @, #, $ の3つのエラー
}

#[test]
fn test_parser_error_handling() {
    // 構文エラーを含むソースコード
    let source = r#"
fn main() {
    let x = ;  // 式がない
}
"#;

    let state = CompilationState::new_from_string("test.yuni", source.to_string()).unwrap();
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // レキシカル解析と構文解析を実行
    let tokens = pipeline.tokenize();
    let ast = pipeline.parse(tokens);

    // ASTが生成されず、エラーが収集されていることを確認
    assert!(ast.is_none());
    assert!(pipeline.state().has_errors());
}

#[test]
fn test_multiple_error_accumulation() {
    // 複数のエラーを含むソースコード
    let source = r#"
fn main() {
    let x = @#$;  // 不正なトークン1
    let y = %^&;  // 不正なトークン2
    let z = !@#;  // 不正なトークン3
}
"#;

    let state = CompilationState::new_from_string("test.yuni", source.to_string()).unwrap();
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // レキシカル解析を実行
    let _tokens = pipeline.tokenize();

    // 複数のエラーが収集されていることを確認
    assert!(pipeline.state().has_errors());
    assert!(pipeline.state().error_count() >= 3);
}

#[test]
fn test_analyzer_error_handling() {
    // セマンティックエラーを含むソースコード
    let source = r#"
package test

fn main() {
    let x = y;  // 未定義の変数を使用
}
"#;

    let state = CompilationState::new_from_string("test.yuni", source.to_string()).unwrap();
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // パイプライン全体を実行
    let _result = pipeline.run();

    // エラーが収集されていることを確認
    assert!(pipeline.state().has_errors());
}

#[test]
fn test_error_type_conversion() {
    // エラー型の変換が正しく動作することを確認
    let lexer_error = YuniError::Lexer(LexerError::UnrecognizedToken {
        token: "@#$".to_string(),
        span: yunilang::ast::Span::new(10, 13),
    });

    // エラーメッセージが適切に生成されることを確認
    let error_message = lexer_error.to_string();
    assert!(error_message.contains("字句解析エラー"));

    let parser_error = YuniError::Parser(ParserError::UnexpectedToken {
        expected: "identifier".to_string(),
        found: "keyword".to_string(),
        span: yunilang::ast::Span::new(20, 27),
    });

    let error_message = parser_error.to_string();
    assert!(error_message.contains("構文解析エラー"));
}
