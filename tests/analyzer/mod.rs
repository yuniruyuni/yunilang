//! セマンティック解析テストの共通モジュール
//! 
//! セマンティック解析テストで使用する共通のヘルパー関数と型を定義する。

use yunilang::analyzer::SemanticAnalyzer;
use yunilang::error::{YuniError, AnalyzerError};
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;
use yunilang::ast::*;

/// ソースコードを解析してASTを取得し、セマンティック解析を実行するヘルパー関数
pub fn analyze_source(source: &str) -> Result<Program, YuniError> {
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing should succeed");
    
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;
    Ok(ast)
}

/// 解析に成功することを確認するヘルパー関数
pub fn assert_analysis_success(source: &str) -> Program {
    analyze_source(source).expect("Analysis should succeed")
}

/// 解析に失敗することを確認するヘルパー関数
pub fn assert_analysis_error(source: &str) {
    assert!(analyze_source(source).is_err(), "Analysis should fail");
}

/// 特定のエラータイプが発生することを確認するヘルパー関数
pub fn assert_specific_error<F>(source: &str, check: F) 
where F: Fn(&AnalyzerError) -> bool {
    let result = analyze_source(source);
    assert!(result.is_err(), "Analysis should fail");
    if let Err(YuniError::Analyzer(error)) = result {
        assert!(check(&error), "Expected specific error type, got: {:?}", error);
    } else if let Err(e) = result {
        panic!("Expected AnalyzerError, got: {:?}", e);
    }
}

// サブモジュールの宣言
#[cfg(test)]
mod type_checking_test;
#[cfg(test)]
mod scope_test;
#[cfg(test)]
mod struct_enum_test;
#[cfg(test)]
mod control_flow_test;
#[cfg(test)]
mod error_variable_function_test;
#[cfg(test)]
mod error_type_test;
#[cfg(test)]
mod method_test;
#[cfg(test)]
mod advanced_test;